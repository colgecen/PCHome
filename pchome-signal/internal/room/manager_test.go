package room_test

import (
	"testing"
	"time"

	"github.com/colgecen/pchome/pchome-signal/internal/room"
	"go.uber.org/zap"
)

func TestCreateRoom(t *testing.T) {
	logger, _ := zap.NewDevelopment()
	manager := room.NewManager(logger)

	pin, err := manager.Create("client-1")
	if err != nil {
		t.Fatalf("Create failed: %v", err)
	}

	if len(pin) != 6 {
		t.Fatalf("Expected 6-digit PIN, got %s", pin)
	}

	r, ok := manager.Get(pin)
	if !ok {
		t.Fatalf("Room not found after creation")
	}

	if r.ClientID != "client-1" {
		t.Fatalf("Expected client-1, got %s", r.ClientID)
	}
}

func TestRoomEviction(t *testing.T) {
	logger, _ := zap.NewDevelopment()
	manager := room.NewManagerWithTTL(logger, 100*time.Millisecond)

	pin, err := manager.Create("client-2")
	if err != nil {
		t.Fatalf("Create failed: %v", err)
	}

	_, ok := manager.Get(pin)
	if !ok {
		t.Fatalf("Room should exist immediately")
	}

	time.Sleep(150 * time.Millisecond)

	_, ok = manager.Get(pin)
	if ok {
		t.Fatalf("Room should have been evicted")
	}
}

func TestRoomLimit(t *testing.T) {
	logger, _ := zap.NewDevelopment()
	manager := room.NewManagerWithTTL(logger, 5*time.Minute)

	for i := 0; i < 10000; i++ {
		_, err := manager.Create("client-bulk")
		if err != nil {
			t.Fatalf("Create failed at iteration %d: %v", i, err)
		}
	}

	_, err := manager.Create("client-overflow")
	if err == nil {
		t.Fatalf("Expected error when room limit reached")
	}
}

func TestDeleteRoom(t *testing.T) {
	logger, _ := zap.NewDevelopment()
	manager := room.NewManager(logger)

	pin, _ := manager.Create("client-3")
	manager.Delete(pin)

	_, ok := manager.Get(pin)
	if ok {
		t.Fatalf("Room should be deleted")
	}
}
