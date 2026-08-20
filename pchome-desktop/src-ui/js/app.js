(function(){
  const pinEl = document.getElementById('pin');
  const statusEl = document.getElementById('status');
  // stub: fetch pin from backend API in real implementation
  async function fetchPin(){
    // placeholder
    return '000000';
  }
  fetchPin().then(p=>{ pinEl.textContent = p; statusEl.textContent = 'PIN generated' });
})();
