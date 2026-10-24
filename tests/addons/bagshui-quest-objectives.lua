-- Offline regression fixture. Load this, Bagshui's ActiveQuestItems.lua, then run the test.
_G = getfenv(0)
local objectives = {}
Bagshui = { currentCharacterData = {}, components = {} }
function Bagshui:AddComponent(fn) fn() end
function Bagshui:RegisterEvent() end
BsUtil = { TableClear = function(t) for k in pairs(t) do t[k] = nil end end }
function GetNumQuestLogEntries() return 1 end
function GetQuestLogTitle() return "Test Quest", 1, nil, false end
function SelectQuestLogEntry() end
function GetNumQuestLeaderBoards() return table.getn(objectives) end
function GetQuestLogLeaderBoard(i) return objectives[i].text, objectives[i].kind end

function TestBagshuiQuestObjectives()
  local manager = Bagshui.components.ActiveQuestItemManager
  local cases = {
    { text = "Quest Item: 2/5", name = "Quest Item", obtained = 2, needed = 5 },
    { text = "任务物品：0/10", name = "任务物品", obtained = 0, needed = 10 },
    { text = "任务物品：3/3", name = "任务物品", obtained = 3, needed = 3 },
    { text = "Quest Item:2/5", name = "Quest Item", obtained = 2, needed = 5 },
    { text = "  任务物品 ： 1 / 4  ", name = "任务物品", obtained = 1, needed = 4 },
    { text = "任务物品　：　1/4", name = "任务物品", obtained = 1, needed = 4 },
    { text = "Part: Two: 1/2", name = "Part: Two", obtained = 1, needed = 2 },
  }
  for _, case in ipairs(cases) do
    objectives = {{ text = case.text, kind = "item" }}
    manager:Update()
    local item = manager.items[case.name]
    assert(item, "Missing parsed item: " .. case.text)
    assert(item.questName == "Test Quest")
    assert(tonumber(item.obtained) == case.obtained)
    assert(tonumber(item.needed) == case.needed)
  end
  objectives = {
    { text = "Valid Item: 1/2", kind = "item" },
    { text = "未取得任务目标数据", kind = "item" },
    { kind = "item" },
    { text = "任务目标已完成", kind = "object" },
    { text = "Monster: 0/1", kind = "monster" },
    { text = ": 0/1", kind = "item" },
  }
  manager:Update()
  assert(manager.items["Valid Item"], "Valid objectives must survive adjacent invalid ones")
  local count = 0
  for _ in pairs(manager.items) do count = count + 1 end
  assert(count == 1, "Invalid or non-item objectives must not become item keys")
  objectives = {{ text = "Opened Chest：1/1", kind = "object" }}
  manager:Update()
  assert(manager.items["Opened Chest"], "Counted object objectives remain supported")
  assert(not manager.items["Valid Item"], "Old quest entries must be cleared")
  objectives = {}
  manager:Update()
  assert(next(manager.items) == nil)
  return "PASS: English/Chinese objective counts, invalid/pending rows, object objectives and stale entries"
end

-- Standalone usage: lua5.1 tests/addons/bagshui-quest-objectives.lua /path/to/Bagshui
if arg and arg[1] then
  dofile(arg[1] .. "/Components/ActiveQuestItems.lua")
  print(TestBagshuiQuestObjectives())
end
