// Pure business logic — no DOM, no Tauri invoke.
// Imported by index.html at runtime and by tests.

export function coordKey(x, y) {
  return `${x},${y}`;
}

export function parseCoord(key) {
  const [x, y] = key.split(',').map(Number);
  return { x, y };
}

export function coordDisplay(x, y) {
  return `${x}x${y}`;
}

export function makeRoom(x, y) {
  return {
    x,
    y,
    roomId: null,
    title: '',
    notes: '',
    imageDataUrl: null,
    imageFilename: null,
    isTemplate: false,
    templateId: null,
    lastAppliedTemplateId: null,
    complete: false
  };
}

export function generateId() {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 10);
}

export function buildDescription(mapName, roomTitle, x, y, format) {
  const fmt = format || '{MAP} - {TITLE} {COORDS}';
  return fmt
    .replace(/\{MAP\}/g, mapName)
    .replace(/\{TITLE\}/g, roomTitle)
    .replace(/\{COORDS\}/g, coordDisplay(x, y))
    .replace(/\{X\}/g, x)
    .replace(/\{Y\}/g, y);
}

export function buildAscDescription(mapName, roomTitle, x, y, format) {
  return `// ${buildDescription(mapName, roomTitle, x, y, format)}`;
}

export function buildLeaveFunction(dir, adjacentRoomId) {
  return `function room_Leave${dir}()\n{\n  player.ChangeRoomAutoPosition(${adjacentRoomId});\n}`;
}

export function parseLeaveFunction(content, dir) {
  const funcPattern = new RegExp(`function\\s+room_Leave${dir}\\s*\\(\\s*\\)`);
  const match = content.match(funcPattern);
  if (!match) return null;

  const funcStart = match.index;
  const braceStart = content.indexOf('{', funcStart + match[0].length);
  if (braceStart === -1) return null;

  let depth = 1;
  let i = braceStart + 1;
  while (i < content.length && depth > 0) {
    if (content[i] === '{') depth++;
    else if (content[i] === '}') depth--;
    i++;
  }
  if (depth !== 0) return null;

  const funcEnd = i;
  const body = content.substring(braceStart + 1, funcEnd - 1);
  const statements = body
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.length > 0 && !line.startsWith('//'));

  const isSimple =
    statements.length === 0 ||
    (statements.length === 1 &&
      /^player\.ChangeRoomAutoPosition\(\d+\);$/.test(statements[0]));

  return {
    start: funcStart,
    end: funcEnd,
    isSimple,
    fullText: content.substring(funcStart, funcEnd)
  };
}

export function sanitizeFilename(title) {
  let sanitized = title.replace(/[^a-zA-Z0-9 _-]/g, '').replace(/\s+/g, '_');
  if (sanitized.length > 50) sanitized = sanitized.substring(0, 50);
  return sanitized;
}

export function roomOverallType(changes) {
  const isNewRoom = changes.some((c) => c.detail.startsWith('Assigned Room ID'));
  if (isNewRoom) return 'new';
  const types = changes.map((c) => c.type);
  if (types.includes('new') || types.includes('update')) return 'update';
  return 'skip';
}

export const ASC_DIRECTIONS = [
  { name: 'Top', dx: 0, dy: 1, crmEventIndex: 3 },
  { name: 'Bottom', dx: 0, dy: -1, crmEventIndex: 2 },
  { name: 'Left', dx: -1, dy: 0, crmEventIndex: 0 },
  { name: 'Right', dx: 1, dy: 0, crmEventIndex: 1 }
];

export function resolveTemplateSourceRoomId(state, templateId) {
  const template = state.templates[templateId];
  if (!template) return null;
  const map = state.maps[template.sourceMapId];
  if (!map) return null;
  const room = map.rooms[template.sourceCoord];
  if (!room) return null;
  return room.roomId ?? null;
}

/**
 * Assign room IDs to unassigned, non-complete rooms.
 * Pure version: takes state and existing file IDs, returns assignments + mutated state.
 */
export function assignRoomIdsSync(state, existingFileIds) {
  const usedIds = new Set();
  for (const mapId of Object.keys(state.maps)) {
    const rooms = state.maps[mapId].rooms;
    for (const key of Object.keys(rooms)) {
      if (rooms[key].roomId !== null) {
        usedIds.add(rooms[key].roomId);
      }
    }
  }

  const unassigned = [];
  for (const mapId of Object.keys(state.maps)) {
    const rooms = state.maps[mapId].rooms;
    for (const key of Object.keys(rooms)) {
      const room = rooms[key];
      if (room.roomId === null && !room.complete) {
        unassigned.push({ mapId, key, room });
      }
    }
  }

  const fileIds = new Set(existingFileIds);
  const assignments = [];
  let nextId = 1;
  for (const entry of unassigned) {
    while (usedIds.has(nextId) || fileIds.has(nextId)) {
      nextId++;
    }
    entry.room.roomId = nextId;
    usedIds.add(nextId);
    assignments.push({
      mapId: entry.mapId,
      mapName: state.maps[entry.mapId].name,
      coord: entry.key,
      roomId: nextId
    });
    nextId++;
  }

  return assignments;
}
