// Temporary local UI diagnostics: only UI/renderer errors and selected frame properties.
(() => {
  const logs = [];
  const seen = new Set();
  const lootEvents = [];
  const diagnostic = { lootEvents, revision: 'ui-20261024-turtle-shop-models', logs, frames: null };
  window.__wenilla_ui_diagnostic = diagnostic;
  for (const level of ['log', 'warn', 'error']) {
    const original = console[level].bind(console);
    console[level] = (...args) => {
      original(...args);
      const line = args.map(String).join(' ');
      if (level === 'log' && !/^(?:%c)?(?:WARN|ERROR)\b|runtime error|texture miss|body skin composite failed/i.test(line)) return;
      if (!/ui_script|WGSL error|ShaderModule|GPUValidationError|WebGPU|BindGroup|RenderPipeline|DrawIndexed|GPUQueue|texture miss|body skin composite|skin composite|UI texture|sprite.*(?:missing|failed)/i.test(line)) return;
      if (seen.has(line) || logs.length >= 100) return;
      seen.add(line);
      logs.push(line.slice(0, 2500));
    };
  }
  const query = String.raw`
local out = {}
for _, name in ipairs({"UIParent", "MainMenuBar", "ActionButton1", "ActionButton1Icon", "TargetFrame", "CharacterFrame", "GameTooltip", "ContainerFrame1"}) do
  local f = getglobal(name)
  local row = name .. ":exists=" .. tostring(f ~= nil)
  if f then
    row = row .. ",shown=" .. tostring(f:IsShown()) .. ",visible=" .. tostring(f:IsVisible()) .. ",alpha=" .. tostring(f:GetAlpha()) .. ",size=" .. tostring(f:GetWidth()) .. "x" .. tostring(f:GetHeight())
    if f.GetTexture then row = row .. ",texture=" .. tostring(f:GetTexture()) end
  end
  table.insert(out, row)
end
table.insert(out, "action1=" .. tostring(GetActionTexture(1)))
table.insert(out, "bag0=" .. tostring(GetContainerNumSlots(0)))
local ok, extra = pcall(function()
  local lines = {}
  local raceName, raceFile = UnitRace("player")
  table.insert(lines, "appearance=" .. tostring(raceFile) .. ",sex=" .. tostring(UnitSex("player")))
  for _, slot in ipairs({1, 3, 5, 6, 7, 8, 9, 10, 15, 19}) do
    local link = GetInventoryItemLink("player", slot) or ""
    local _, _, id = string.find(link, "item:(%d+)")
    table.insert(lines, "equipment=" .. tostring(slot) .. ",item=" .. tostring(id) .. ",icon=" .. tostring(GetInventoryItemTexture("player", slot)))
  end
  table.insert(lines, "loot=" .. tostring(GetNumLootItems()) .. ",window=" .. tostring(LootFrame and LootFrame:IsVisible()))
  local cvOk, auto = pcall(GetCVar, "autoLootDefault")
  table.insert(lines, "autoLootDefault=" .. tostring(cvOk and auto))
  return table.concat(lines, "\n")
end)
table.insert(out, ok and extra or ("DETAIL_QUERY_ERROR " .. tostring(extra)))
if BenillaScriptLog and BenillaScriptLog.rows then
  for i, row in ipairs(BenillaScriptLog.rows) do
    if i <= 15 then table.insert(out, "SCRIPT_ERROR " .. strsub(tostring(row.message), 1, 1200)) end
  end
end
return table.concat(out, "\n")`;
  let busy = false;
  let lastReport = "";
  async function report() {
    if (busy) return;
    busy = true;
    try {
      if (window.wenilla?.ready) {
        try { diagnostic.frames = await window.wenilla.lua(query, 4000); }
        catch { /* VM can be between glue and world; retry later. */ }
      }
      const payload = { ...diagnostic, logs: logs.slice(0, 24), frames: [...(diagnostic.frames || []), "LOOT_EVENTS " + JSON.stringify(lootEvents)], phase: window.wenilla?.state?.session?.state ?? null };
      let body = JSON.stringify(payload);
      while (new TextEncoder().encode(body).length > 60000 && payload.logs.length) {
        payload.logs.pop();
        body = JSON.stringify(payload);
      }
      if (body === lastReport) return;
      const response = await fetch('/ui-report', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body,
      });
      if (response.ok) lastReport = body;
    } catch { /* Diagnostics must never affect the game. */ }
    finally { busy = false; }
  }
  setInterval(report, 15000);
  const eventTimer = setInterval(() => {
    if (!window.wenilla?.on) return;
    clearInterval(eventTimer);
    for (const event of ['LOOT_OPENED', 'LOOT_CLOSED', 'LOOT_SLOT_CLEARED']) {
      window.wenilla.on(event, args => {
        lootEvents.push({ event, at: Math.round(performance.now()), args });
        if (lootEvents.length > 32) lootEvents.shift();
        setTimeout(report, 300);
      });
    }
  }, 1000);
  window.addEventListener('load', () => {
    const button = document.createElement('button');
    button.textContent = '界面诊断';
    Object.assign(button.style, { position: 'fixed', right: '12px', top: '52px', zIndex: '10000' });
    button.onclick = async () => {
      await report();
      const panel = document.createElement('pre');
      panel.textContent = JSON.stringify(diagnostic, null, 2);
      Object.assign(panel.style, { position: 'fixed', inset: '10%', padding: '20px', zIndex: '10001', background: '#181818', color: '#fff', overflow: 'auto', whiteSpace: 'pre-wrap' });
      panel.title = '点击关闭';
      panel.onclick = () => panel.remove();
      document.body.appendChild(panel);
    };
    document.body.appendChild(button);
  });
})();
