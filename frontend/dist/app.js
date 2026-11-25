const $ = (s) => document.querySelector(s);
const chat = $('#chat'), input = $('#input'), form = $('#form');
const token = $('#token'), modeSel = $('#modeSel');
const dot = $('#dot'), connText = $('#connText'), statusEl = $('#status'), latencyEl = $('#latency');
let history = [];
let base = ''; // same-origin by default; set QIIIN_API_BASE to override

token.value = sessionStorage.getItem('qiiin_token') || '';
token.addEventListener('change', () => sessionStorage.setItem('qiiin_token', token.value));

function add(role, text) {
  const div = document.createElement('div');
  div.className = 'msg ' + (role === 'user' ? 'user' : role === 'system' ? 'system' : 'ai');
  div.textContent = text;
  chat.appendChild(div);
  chat.scrollTop = chat.scrollHeight;
  return div;
}

async function refreshStatus() {
  try {
    const r = await fetch(base + '/health', { cache: 'no-store' });
    const j = await r.json();
    dot.classList.toggle('on', !!j.ok);
    connText.textContent = j.ok ? 'online' : 'offline';
    statusEl.textContent = `status: gateway ${j.version || '?'}`;
  } catch (e) {
    dot.classList.remove('on'); connText.textContent = 'offline';
    statusEl.textContent = 'status: gateway unreachable';
  }
}
refreshStatus();
setInterval(refreshStatus, 15000);

form.addEventListener('submit', async (ev) => {
  ev.preventDefault();
  const text = input.value.trim();
  if (!text) return;
  input.value = '';
  add('user', text);
  history.push({ role: 'user', content: text });
  const out = add('ai', '');
  const btn = form.querySelector('button');
  btn.disabled = true;
  const started = performance.now();
  try {
    const r = await fetch(base + '/v1/chat/completions', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': 'Bearer ' + token.value
      },
      body: JSON.stringify({
        model: 'qwen3-4b',
        messages: history.slice(-12),
        stream: true,
        max_tokens: 512
      })
    });
    if (!r.ok) {
      out.textContent = 'error: HTTP ' + r.status + ' ' + (await r.text()).slice(0, 200);
      return;
    }
    const reader = r.body.getReader();
    const dec = new TextDecoder();
    let buf = '';
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buf += dec.decode(value, { stream: true });
      const lines = buf.split('\n');
      buf = lines.pop() || '';
      for (const line of lines) {
        if (!line.startsWith('data:')) continue;
        const payload = line.slice(5).trim();
        if (!payload || payload === '[DONE]') continue;
        try {
          const j = JSON.parse(payload);
          const delta = j.choices?.[0]?.delta?.content || '';
          out.textContent += delta;
          chat.scrollTop = chat.scrollHeight;
        } catch (_) { /* partial chunk */ }
      }
    }
    const answer = out.textContent.trim();
    if (answer) history.push({ role: 'assistant', content: answer });
    $('#mode').textContent = modeSel.value.toUpperCase();
    latencyEl.textContent = Math.round(performance.now() - started) + ' ms';
  } catch (e) {
    out.textContent = 'error: ' + e.message;
  } finally {
    btn.disabled = false;
  }
});
