package signal

import (
	"strconv"
	"sync/atomic"

	"github.com/prometheus/client_golang/prometheus"
)

var (
	activeRoomsGauge      prometheus.Gauge
	connectedClientsGauge prometheus.Gauge
	messagesRelayedCounter prometheus.Counter
)

func init() {
	activeRoomsGauge = prometheus.NewGauge(prometheus.GaugeOpts{
		Name: "pchome_signal_active_rooms",
		Help: "Number of active PIN rooms",
	})
	connectedClientsGauge = prometheus.NewGauge(prometheus.GaugeOpts{
		Name: "pchome_signal_connected_clients",
		Help: "Number of connected WebSocket clients",
	})
	messagesRelayedCounter = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "pchome_signal_messages_relayed_total",
		Help: "Total number of WebSocket messages relayed",
	})

	prometheus.MustRegister(activeRoomsGauge, connectedClientsGauge, messagesRelayedCounter)
}

func IncActiveRooms() {
	activeRoomsGauge.Inc()
}

func DecActiveRooms() {
	activeRoomsGauge.Dec()
}

func SetActiveRooms(v float64) {
	activeRoomsGauge.Set(v)
}

func IncConnectedClients() {
	connectedClientsGauge.Inc()
}

func DecConnectedClients() {
	connectedClientsGauge.Dec()
}

func IncMessagesRelayed() {
	messagesRelayedCounter.Inc()
}

func IncMessagesRelayedBy(n int64) {
	for i := int64(0); i < n; i++ {
		messagesRelayedCounter.Inc()
	}
}

func GetActiveRooms() float64 {
	return activeRoomsGauge.Get()
}

func GetConnectedClients() float64 {
	return connectedClientsGauge.Get()
}

func GetMessagesRelayed() uint64 {
	return uint64(messagesRelayedCounter.Get())
}

func GetActiveRoomsString() string {
	return strconv.FormatFloat(GetActiveRooms(), 'f', 0, 64)
}

func GetConnectedClientsString() string {
	return strconv.FormatFloat(GetConnectedClients(), 'f', 0, 64)
}

func GetMessagesRelayedString() string {
	return strconv.FormatUint(GetMessagesRelayed(), 10)
}
