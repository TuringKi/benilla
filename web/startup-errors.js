// Keep fatal startup errors visible when the render loop cannot draw its own UI.
(() => {
  let priority = 0;
  function show(error, level = 1) {
    if (level < priority) return;
    priority = level;
    const message = error?.stack || error?.message || String(error);
    window.__wenilla_last_error = message;
    let panel = document.getElementById('startup-error');
    if (!panel) {
      panel = document.createElement('div');
      panel.id = 'startup-error';
      panel.setAttribute('role', 'alert');
      Object.assign(panel.style, {
        position: 'fixed', inset: '24px', zIndex: '100000', padding: '20px',
        color: '#fff', background: '#291719', border: '1px solid #cb7373',
        font: '14px/1.5 system-ui', overflow: 'auto', whiteSpace: 'pre-wrap',
      });
      (document.body || document.documentElement).appendChild(panel);
    }
    panel.textContent = '客户端启动失败\n\n' + message.slice(0, 5000) +
      '\n\n请将上面的报错文字发给我，以便继续定位。';
  }
  window.addEventListener('error', event => {
    if (event.message) show(event.error || event.message);
  });
  window.addEventListener('unhandledrejection', event => show(event.reason));
  const originalError = console.error.bind(console);
  console.error = (...args) => {
    originalError(...args);
    const message = args.map(String).join(' ');
    if (message.includes('panicked at')) show(message, 2);
  };
  if (!window.isSecureContext) {
    show('WebGPU 需要安全连接。请使用 HTTPS 地址，并在访问的电脑上信任局域网 CA 证书。');
  } else if (!navigator.gpu) {
    show('当前浏览器没有提供 WebGPU。请检查浏览器的硬件加速和 WebGPU 支持。');
  }
})();
