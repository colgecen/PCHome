package signal

import (
	"testing"
	"time"

	"github.com/colgecen/pchome/pchome-signal/internal/room"
	"go.uber.org/zap"
)

// TestRelayRoundtripNoEcho verifies that a message broadcast by one client is
// delivered to the peer sharing the same PIN but NOT echoed back to the sender.
func TestRelayRoundtripNoEcho(t *testing.T) {
	logger, _ := zap.NewDevelopment()
	rooms := room.NewManager(logger)
	metrics := NewMetrics()
	hub := NewHub(rooms, metrics, logger)

	go hub.Run()

	makeClient := func(role string) *Client {
		return &Client{
			ID:     role + "-id",
			PIN:    "123456",
			Role:   role,
			Send:   make(chan []byte, 16),
			Hub:    hub,
			Logger: logger,
		}
	}

	desktop := makeClient("desktop")
	mobile := makeClient("mobile")

	hub.register <- desktop
	hub.register <- mobile

	// Give the hub a moment to register both clients.
	time.Sleep(20 * time.Millisecond)

	hub.broadcast <- &relayMessage{
		from: desktop,
		msg:  &Message{Type: "123456", Data: "sdp-offer"},
	}

	select {
	case got := <-mobile.Send:
		if string(got) != "sdp-offer" {
			t.Fatalf("mobile received wrong payload: %q", got)
		}
	case <-time.After(time.Second):
		t.Fatalf("mobile never received the relayed message")
	}

	select {
	case got := <-desktop.Send:
		t.Fatalf("sender unexpectedly received its own message: %q", got)
	case <-time.After(50 * time.Millisecond):
		// Correct: no echo to sender.
	}
}
