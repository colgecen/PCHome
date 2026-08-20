(function() {
  'use strict';

  const State = {
    IDLE: 'idle',
    GENERATING_PIN: 'generating_pin',
    PIN_READY: 'pin_ready',
    CONNECTING: 'connecting',
    CONNECTED: 'connected',
    ERROR: 'error',
  };

  let currentState = State.IDLE;
  let reconnectAttempts = 0;
  const MAX_RECONNECT_ATTEMPTS = 5;
  const RECONNECT_DELAY_MS = 2000;

  const pinEl = document.getElementById('pin');
  const statusEl = document.getElementById('status');
  const errorSection = document.getElementById('error-section');
  const errorMessage = document.getElementById('error-message');
  const connectionIndicator = document.getElementById('connection-indicator');
  const hud = document.getElementById('hud');

  function setState(newState) {
    currentState = newState;
    render();
  }

  function showError(message) {
    errorMessage.textContent = message;
    errorSection.hidden = false;
    setState(State.ERROR);
  }

  function hideError() {
    errorSection.hidden = true;
    errorMessage.textContent = '';
  }

  async function generatePin() {
    hideError();
    setState(State.GENERATING_PIN);
    statusEl.textContent = 'Generating PIN...';
    connectionIndicator.textContent = 'Generating PIN';
    connectionIndicator.className = 'disconnected';

    try {
      const response = await fetch('/api/pin/generate', { method: 'POST' });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const data = await response.json();
      pinEl.textContent = data.pin;
      statusEl.textContent = 'PIN ready';
      connectionIndicator.textContent = 'PIN ready';
      connectionIndicator.className = 'disconnected';
      setState(State.PIN_READY);
      setTimeout(startConnection, 5000);
    } catch (err) {
      showError('Failed to generate PIN: ' + err.message);
      connectionIndicator.textContent = 'Error';
      connectionIndicator.className = 'disconnected';
    }
  }

  async function startConnection() {
    if (currentState === State.CONNECTING || currentState === State.CONNECTED) return;

    hideError();
    setState(State.CONNECTING);
    statusEl.textContent = 'Connecting...';
    connectionIndicator.textContent = 'Connecting...';
    connectionIndicator.className = 'disconnected';

    try {
      const response = await fetch('/api/connection/connect', { method: 'POST' });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const data = await response.json();

      if (data.state === 'connected') {
        statusEl.textContent = 'Connected';
        statusEl.className = 'connected';
        connectionIndicator.textContent = 'Connected';
        connectionIndicator.className = 'connected';
        setState(State.CONNECTED);
        reconnectAttempts = 0;
      } else {
        throw new Error(data.error || 'Connection failed');
      }
    } catch (err) {
      showError('Connection failed: ' + err.message);
      statusEl.textContent = 'Connection failed';
      statusEl.className = 'error';
      connectionIndicator.textContent = 'Disconnected';
      connectionIndicator.className = 'disconnected';
      attemptReconnect();
    }
  }

  async function attemptReconnect() {
    if (reconnectAttempts >= MAX_RECONNECT_ATTEMPTS) {
      showError('Max reconnection attempts reached. Please restart.');
      return;
    }

    reconnectAttempts++;
    statusEl.textContent = `Reconnecting... (${reconnectAttempts}/${MAX_RECONNECT_ATTEMPTS})`;
    await new Promise(r => setTimeout(r, RECONNECT_DELAY_MS));
    await startConnection();
  }

  function render() {
    switch (currentState) {
      case State.IDLE:
        pinEl.textContent = '------';
        statusEl.textContent = 'Idle';
        statusEl.className = '';
        connectionIndicator.textContent = 'Disconnected';
        connectionIndicator.className = 'disconnected';
        break;
      case State.GENERATING_PIN:
        statusEl.textContent = 'Generating PIN...';
        statusEl.className = 'warning';
        break;
      case State.PIN_READY:
        statusEl.textContent = 'PIN ready';
        statusEl.className = '';
        break;
      case State.CONNECTING:
        statusEl.textContent = 'Connecting...';
        statusEl.className = 'warning';
        break;
      case State.CONNECTED:
        statusEl.textContent = 'Connected';
        statusEl.className = 'connected';
        connectionIndicator.textContent = 'Connected';
        connectionIndicator.className = 'connected';
        break;
      case State.ERROR:
        statusEl.textContent = 'Error';
        statusEl.className = 'error';
        connectionIndicator.textContent = 'Error';
        connectionIndicator.className = 'disconnected';
        break;
    }
  }

  async function pollStatus() {
    try {
      const response = await fetch('/api/status');
      if (!response.ok) return;
      const data = await response.json();

      if (data.pin) {
        pinEl.textContent = data.pin;
      }

      if (data.status) {
        statusEl.textContent = data.status;
        statusEl.className = data.status === 'connected' ? 'connected' : '';
      }

      if (data.connected) {
        connectionIndicator.textContent = 'Connected';
        connectionIndicator.className = 'connected';
        setState(State.CONNECTED);
      } else if (data.error) {
        showError(data.error);
      }
    } catch {
      // ignore poll errors
    }
  }

  document.addEventListener('keydown', (e) => {
    if (e.ctrlKey && e.shiftKey && e.key === 'P') {
      e.preventDefault();
      if (currentState === State.IDLE || currentState === State.ERROR) {
        generatePin();
      }
    }

    if (e.key === 'Escape') {
      e.preventDefault();
      hideError();
      if (currentState === State.ERROR) {
        setState(State.IDLE);
      }
    }

    if (e.key === 'r' && e.ctrlKey) {
      e.preventDefault();
      if (currentState === State.ERROR) {
        startConnection();
      }
    }
  });

  hud.addEventListener('click', () => {
    hud.focus();
  });

  setInterval(pollStatus, 2000);
  render();
})();
