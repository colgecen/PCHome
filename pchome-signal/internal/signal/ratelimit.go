package signal

import (
	"net/http"
	"sync"
	"time"
)

type RateLimiter struct {
	mu       sync.Mutex
	clients  map[string]*clientState
	limit    int
	window   time.Duration
}

type clientState struct {
	count    int
	lastSeen time.Time
}

func NewRateLimiter(limit int, window time.Duration) *RateLimiter {
	rl := &RateLimiter{
		clients: make(map[string]*clientState),
		limit:  limit,
		window: window,
	}
	go rl.cleanupLoop()
	return rl
}

func (rl *RateLimiter) cleanupLoop() {
	ticker := time.NewTicker(rl.window)
	defer ticker.Stop()
	for range ticker.C {
		rl.mu.Lock()
		now := time.Now()
		for ip, state := range rl.clients {
			if now.Sub(state.lastSeen) > rl.window {
				delete(rl.clients, ip)
			}
		}
		rl.mu.Unlock()
	}
}

func (rl *RateLimiter) Allow(ip string) bool {
	rl.mu.Lock()
	defer rl.mu.Unlock()

	now := time.Now()
	state, exists := rl.clients[ip]
	if !exists || now.Sub(state.lastSeen) > rl.window {
		rl.clients[ip] = &clientState{count: 1, lastSeen: now}
		return true
	}

	state.count++
	state.lastSeen = now
	return state.count <= rl.limit
}

func RateLimitMiddleware(rl *RateLimiter, next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		ip := r.RemoteAddr
		if !rl.Allow(ip) {
			w.WriteHeader(http.StatusTooManyRequests)
			w.Write([]byte("rate limit exceeded"))
			return
		}
		next.ServeHTTP(w, r)
	})
}
