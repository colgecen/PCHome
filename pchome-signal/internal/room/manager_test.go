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

// TestPINFormat asserts every generated PIN is exactly 6 ASCII digits, which is
// required by the spec and by the mobile client's parsing.
func TestPINFormat(t *testing.T) {
	logger, _ := zap.NewDevelopment()
	manager := room.NewManager(logger)

	for i := 0; i < 200; i++ {
		pin, err := manager.Create("format-client")
		if err != nil {
			t.Fatalf("Create failed: %v", err)
		}
		if len(pin) != 6 {
			t.Fatalf("expected 6-digit PIN, got %q (len %d)", pin, len(pin))
		}
		for _, c := range pin {
			if c < '0' || c > '9' {
				t.Fatalf("PIN %q contains non-digit character %q", pin, c)
			}
		}
		manager.Delete(pin)
	}
}

// TestReserveRejectsMalformedPIN verifies the desktop-supplied PIN is validated.
func TestReserveRejectsMalformedPIN(t *testing.T) {
	logger, _ := zap.NewDevelopment()
	manager := room.NewManager(logger)

	if err := manager.Reserve("123", "c"); err == nil {
		t.Fatalf("expected error for too-short PIN")
	}
	if err := manager.Reserve("12a456", "c"); err == nil {
		t.Fatalf("expected error for non-digit PIN")
	}
	if err := manager.Reserve("123456", "c"); err != nil {
		t.Fatalf("expected success for valid PIN, got %v", err)
	}
	if err := manager.Reserve("123456", "c2"); err == nil {
		t.Fatalf("expected error for duplicate PIN")
	}
}
