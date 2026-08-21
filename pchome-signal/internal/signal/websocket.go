package signal

import (
	"net/http"
	"sync"
	"time"

	"github.com/colgecen/pchome/pchome-signal/internal/room"
	"github.com/google/uuid"
	"github.com/gorilla/websocket"
	"go.uber.org/zap"
)

var upgrader = websocket.Upgrader{
	// Bound inbound read buffer to 1 MiB to prevent a memory-exhaustion DoS
	// on the relay.
	ReadBufferSize: 1 << 20,
	CheckOrigin: func(r *http.Request) bool {
		return true
	},
}

type Message struct {
	Type string `json:"type"`
	Data string `json:"data"`
}

type Hub struct {
	rooms   *room.Manager
	log     *zap.Logger
	clients map[*Client]bool
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

// relayMsgPool reuses relayMessage allocations on the hot relay path. Safe
// because the hub fully consumes a relayMessage within a single synchronous
// Run iteration before the next one is dequeued.
var relayMsgPool = sync.Pool{New: func() any { return &relayMessage{} }}

type Client struct {
	ID     string
	PIN    string
	Role   string
	Conn   *websocket.Conn
	Send   chan []byte
	Hub    *Hub
	Logger *zap.Logger
	// closeOnce guarantees the Send channel is closed exactly once, avoiding a
	// "close of closed channel" panic when both the hub unregister path and the
	// slow-consumer eviction path try to close it.
	closeOnce sync.Once
}

func NewHub(rooms *room.Manager, log *zap.Logger) *Hub {
	return &Hub{
		rooms:      rooms,
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
			IncConnectedClients()
			client.Logger.Info("Client registered", zap.String("id", client.ID), zap.String("role", client.Role))
		case client := <-h.unregister:
			h.mu.Lock()
			if _, ok := h.clients[client]; ok {
				delete(h.clients, client)
				client.closeSend()
			}
			h.mu.Unlock()
			DecConnectedClients()
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
					IncMessagesRelayed()
					h.rooms.Touch(rm.msg.Type)
				default:
					delete(h.clients, client)
					client.closeSend()
				}
			}
			h.mu.RUnlock()
			relayMsgPool.Put(rm)
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

		rm := relayMsgPool.Get().(*relayMessage)
		rm.from = c
		rm.msg = &Message{Type: c.PIN, Data: string(message)}
		c.Hub.broadcast <- rm
	}
}

// closeSend closes the client's Send channel exactly once.
func (c *Client) closeSend() {
	c.closeOnce.Do(func() {
		close(c.Send)
	})
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
