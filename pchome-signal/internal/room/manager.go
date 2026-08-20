package room

import (
	"fmt"
	"math/rand"
	"sync"
	"time"

	"go.uber.org/zap"
)

const (
	DefaultPINLength = 6
	DefaultTTL       = 5 * time.Minute
	MaxRooms         = 10000
)

type Room struct {
	PIN      string
	ClientID string
	Created  time.Time
}

type Manager struct {
	rooms map[string]*Room
	mu    sync.RWMutex
	ttl   time.Duration
	log   *zap.Logger
}

func NewManager(log *zap.Logger) *Manager {
	return NewManagerWithTTL(log, DefaultTTL)
}

func NewManagerWithTTL(log *zap.Logger, ttl time.Duration) *Manager {
	m := &Manager{
		rooms: make(map[string]*Room),
		ttl:   ttl,
		log:   log,
	}
	go m.gcLoop()
	return m
}

func (m *Manager) Create(clientID string) (string, error) {
	m.mu.Lock()
	defer m.mu.Unlock()

	if len(m.rooms) >= MaxRooms {
		return "", fmt.Errorf("room limit reached")
	}

	var pin string
	for {
		pin = generatePIN()
		if _, exists := m.rooms[pin]; !exists {
			break
		}
	}

	m.rooms[pin] = &Room{
		PIN:      pin,
		ClientID: clientID,
		Created:  time.Now(),
	}

	m.log.Info("Room created", zap.String("pin", pin), zap.String("clientID", clientID))
	return pin, nil
}

func (m *Manager) Get(pin string) (*Room, bool) {
	m.mu.RLock()
	defer m.mu.RUnlock()
	room, ok := m.rooms[pin]
	return room, ok
}

func (m *Manager) Delete(pin string) {
	m.mu.Lock()
	defer m.mu.Unlock()
	delete(m.rooms, pin)
}

func (m *Manager) gcLoop() {
	ticker := time.NewTicker(30 * time.Second)
	defer ticker.Stop()

	for range ticker.C {
		m.evict()
	}
}

func (m *Manager) evict() {
	m.mu.Lock()
	defer m.mu.Unlock()

	now := time.Now()
	var evicted int
	for pin, room := range m.rooms {
		if now.Sub(room.Created) > m.ttl {
			delete(m.rooms, pin)
			evicted++
		}
	}

	if evicted > 0 {
		m.log.Info("Evicted expired rooms", zap.Int("count", evicted))
	}
}

func generatePIN() string {
	const digits = "0123456789"
	pin := make([]byte, DefaultPINLength)
	for i := range pin {
		pin[i] = digits[rand.Intn(len(digits))]
	}
	return string(pin)
}
