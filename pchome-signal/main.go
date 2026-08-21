package main

import (
	"crypto/tls"
	"flag"
	"fmt"
	"net/http"
	"os"
	"os/signal"
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
	metrics := signal.NewMetrics()
	hub := signal.NewHub(roomManager, metrics, logger)

	go hub.Run()

	mux := http.NewServeMux()
	mux.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		w.Write([]byte("ok"))
	})
	mux.Handle("/metrics", promhttp.Handler())
	mux.HandleFunc("/ws", func(w http.ResponseWriter, r *http.Request) {
		signal.ServeWs(hub, w, r)
	})

	handler := mux
	if *rateLimit > 0 {
		limiter := signal.NewRateLimiter(*rateLimit, time.Minute)
		handler = signal.RateLimitMiddleware(limiter, mux).(http.Handler)
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
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
	<-quit

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	if err := server.Shutdown(ctx); err != nil {
		logger.Fatal("Server forced to shutdown", zap.Error(err))
	}

	logger.Info("PChome Signal server stopped")
}
