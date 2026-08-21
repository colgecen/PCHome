package main

import (
	"context"
	"crypto/tls"
	"flag"
	"net/http"
	"os"
	osignal "os/signal"
	"syscall"
	"time"

	"github.com/colgecen/pchome/pchome-signal/internal/room"
	"github.com/colgecen/pchome/pchome-signal/internal/signal"
	"github.com/prometheus/client_golang/prometheus/promhttp"
	"go.uber.org/zap"
)

func main() {
	var (
		addr        = flag.String("addr", ":8080", "listen address")
		tlsCert     = flag.String("tls-cert", "", "TLS certificate file path")
		tlsKey      = flag.String("tls-key", "", "TLS private key file path")
		rateLimit   = flag.Int("rate-limit", 20, "max PIN attempts per IP per minute")
		pinTTL      = flag.Duration("pin-ttl", 5*time.Minute, "PIN time-to-live")
	)
	flag.Parse()

	logger, err := zap.NewProduction()
	if err != nil {
		panic(err)
	}
	defer logger.Sync()

	roomManager := room.NewManagerWithTTL(logger, *pinTTL)
	hub := signal.NewHub(roomManager, logger)

	go hub.Run()

	go func() {
		ticker := time.NewTicker(10 * time.Second)
		defer ticker.Stop()
		for range ticker.C {
			signal.SetActiveRooms(float64(roomManager.Count()))
		}
	}()

	mux := http.NewServeMux()
	mux.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		w.Write([]byte("ok"))
	})
	mux.Handle("/metrics", promhttp.Handler())
	mux.HandleFunc("/ws", func(w http.ResponseWriter, r *http.Request) {
		signal.ServeWs(hub, w, r)
	})

	// CORS middleware so the browser-based HUD (served from a different origin)
	// can open a WebSocket and scrape /metrics.
	handler := withCORS(mux)
	if *rateLimit > 0 {
		limiter := signal.NewRateLimiter(*rateLimit, time.Minute)
		handler = signal.RateLimitMiddleware(limiter, handler)
		logger.Info("rate limiting enabled", zap.Int("limit", *rateLimit), zap.Duration("window", time.Minute))
	}

	server := &http.Server{
		Addr:    *addr,
		Handler: handler,
	}

	go func() {
		logger.Info("PChome Signal server starting", zap.String("addr", server.Addr))
		var listenErr error
		if *tlsCert != "" && *tlsKey != "" {
			server.TLSConfig = &tls.Config{MinVersion: tls.VersionTLS12}
			listenErr = server.ListenAndServeTLS(*tlsCert, *tlsKey)
			logger.Info("TLS enabled", zap.String("cert", *tlsCert))
		} else {
			listenErr = server.ListenAndServe()
			logger.Warn("TLS disabled - use reverse proxy for production")
		}
		if listenErr != nil && listenErr != http.ErrServerClosed {
			logger.Fatal("Server failed", zap.Error(listenErr))
		}
	}()

	quit := make(chan os.Signal, 1)
	osignal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
	<-quit

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	if err := server.Shutdown(ctx); err != nil {
		logger.Fatal("Server forced to shutdown", zap.Error(err))
	}

	logger.Info("PChome Signal server stopped")
}

// withCORS adds permissive CORS headers required for the browser HUD to open a
// WebSocket against the server and to scrape /metrics from a different origin.
// In production this should be locked down to known origins via configuration.
func withCORS(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Access-Control-Allow-Origin", "*")
		w.Header().Set("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
		w.Header().Set("Access-Control-Allow-Headers", "Content-Type, Authorization")
		if r.Method == http.MethodOptions {
			w.WriteHeader(http.StatusNoContent)
			return
		}
		next.ServeHTTP(w, r)
	})
}
