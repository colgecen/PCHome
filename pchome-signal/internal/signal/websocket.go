package signal

import (
	"encoding/json"
	"net/http"
	"sync"
	"time"

	"github.com/colgecen/pchome/pchome-signal/internal/room"
	"github.com/google/uuid"
	"github.com/gorilla/websocket"
	"go.uber.org/zap"
)

var upgrader = websocket.Upgrader{
	CheckOrigin: func(r *http.Request) bool {
		return true
	},
}

type Message struct {
	Type string `json:"type"`
	Data string `json:"data"`
}

type Hub struct {
	rooms      *room.Manager
	metrics    *Metrics
	log        *zap.Logger
	clients    map[*Client]bool
	register   chan *Client
	unregister chan *Client
	broadcast  chan *relayMessage
	mu         sync.RWMutex
}

// relayMessage carries a payload plus the sender so the hub can avoid echoing
// the message back to the originator.
type relayMessage struct {
	from *Client
	msg  *Message
}

type Client struct {
	ID     string
	PIN    string
	Role   string
	Conn   *websocket.Conn
	Send   chan []byte
	Hub    *Hub
	Logger *zap.Logger
}

func NewHub(rooms *room.Manager, metrics *Metrics, log *zap.Logger) *Hub {
	return &Hub{
		rooms:      rooms,
		metrics:    metrics,
		log:        log,
		clients:    make(map[*Client]bool),
		register:   make(chan *Client),
		unregister: make(chan *Client),
		broadcast:  make(chan *relayMessage),
	}
}

func (h *Hub) Run() {
	for {
		select {
		case client := <-h.register:
			h.mu.Lock()
			h.clients[client] = true
			h.mu.Unlock()
			h.metrics.IncConnectedClients()
			client.Logger.Info("Client registered", zap.String("id", client.ID), zap.String("role", client.Role))
		case client := <-h.unregister:
			h.mu.Lock()
			if _, ok := h.clients[client]; ok {
				delete(h.clients, client)
				close(client.Send)
			}
			h.mu.Unlock()
			h.metrics.DecConnectedClients()
			client.Logger.Info("Client unregistered", zap.String("id", client.ID))
		case rm := <-h.broadcast:
			h.mu.RLock()
			for client := range h.clients {
				if client == rm.from {
					continue
				}
				if client.PIN != rm.msg.Type {
					continue
				}
				select {
				case client.Send <- []byte(rm.msg.Data):
					h.metrics.IncMessagesRelayed()
				default:
					close(client.Send)
					delete(h.clients, client)
				}
			}
			h.mu.RUnlock()
		}
	}
}

func ServeWs(hub *Hub, w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		return
	}

	pin := r.URL.Query().Get("pin")
	role := r.URL.Query().Get("role")
	if pin == "" || (role != "desktop" && role != "mobile") {
		conn.WriteJSON(map[string]string{"error": "missing pin or role"})
		conn.Close()
		return
	}

	clientID := uuid.New().String()
	if role == "desktop" {
		if err := hub.rooms.Reserve(pin, clientID); err != nil {
			conn.WriteJSON(map[string]string{"error": err.Error()})
			conn.Close()
			return
		}
	} else {
		if _, exists := hub.rooms.Get(pin); !exists {
			conn.WriteJSON(map[string]string{"error": "invalid pin"})
			conn.Close()
			return
		}
	}

	logger := zap.NewExample()
	client := &Client{
		ID:     clientID,
		PIN:    pin,
		Role:   role,
		Conn:   conn,
		Send:   make(chan []byte, 256),
		Hub:    hub,
		Logger: logger,
	}

	hub.register <- client

	go client.writePump()
	go client.readPump()
}

func (c *Client) readPump() {
	defer func() {
		c.Hub.unregister <- c
		c.Conn.Close()
	}()

	c.Conn.SetReadDeadline(time.Now().Add(60 * time.Second))
	c.Conn.SetPongHandler(func(string) error {
		c.Conn.SetReadDeadline(time.Now().Add(60 * time.Second))
		return nil
	})

	for {
		_, message, err := c.Conn.ReadMessage()
		if err != nil {
			if websocket.IsUnexpectedCloseError(err, websocket.CloseGoingAway, websocket.CloseAbnormalClosure) {
				c.Logger.Error("WebSocket read error", zap.Error(err))
			}
			break
		}

		c.Hub.broadcast <- &relayMessage{
			from: c,
			msg:  &Message{Type: c.PIN, Data: string(message)},
		}
	}
}

func (c *Client) writePump() {
	ticker := time.NewTicker(54 * time.Second)
	defer func() {
		ticker.Stop()
		c.Conn.Close()
	}()

	for {
		select {
		case message, ok := <-c.Send:
			if !ok {
				c.Conn.WriteMessage(websocket.CloseMessage, []byte{})
				return
			}
			if err := c.Conn.WriteMessage(websocket.TextMessage, message); err != nil {
				return
			}
		case <-ticker.C:
			if err := c.Conn.WriteMessage(websocket.PingMessage, nil); err != nil {
				return
			}
		}
	}
}
