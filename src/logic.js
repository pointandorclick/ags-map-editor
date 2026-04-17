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
    complete: false,
    blockedEdges: {}
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

/**
 * Remove a simple room_LeaveXxx function from script content.
 * Returns { content, removed }. Only removes if the function is "simple"
 * (contains only a player.ChangeRoomAutoPosition call).
 */
export function removeSimpleLeaveFunction(content, dir) {
  const parsed = parseLeaveFunction(content, dir);
  if (!parsed) return { content, removed: false };
  if (!parsed.isSimple) return { content, removed: false };
  // Trim preceding blank lines
  let start = parsed.start;
  while (start > 0 && content[start - 1] === '\n') start--;
  if (start > 0) start++; // keep one newline separator
  const newContent = content.substring(0, start) + content.substring(parsed.end);
  return { content: newContent.replace(/\n{3,}/g, '\n\n').trimEnd() + '\n', removed: true };
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
  if (types.includes('error')) return 'error';
  if (types.includes('new') || types.includes('update')) return 'update';
  return 'skip';
}

export const ASC_DIRECTIONS = [
  { name: 'Top', dx: 0, dy: 1, crmEventIndex: 3 },
  { name: 'Bottom', dx: 0, dy: -1, crmEventIndex: 2 },
  { name: 'Left', dx: -1, dy: 0, crmEventIndex: 0 },
  { name: 'Right', dx: 1, dy: 0, crmEventIndex: 1 }
];

/**
 * Mark a room as a template and add it to the templates collection.
 * Returns the new template ID, or null if the room has no image.
 */
export function markRoomAsTemplate(state, mapId, coordKeyStr, templateName) {
  const map = state.maps[mapId];
  if (!map) return null;
  const room = map.rooms[coordKeyStr];
  if (!room || !room.imageDataUrl) return null;

  const id = generateId();
  room.isTemplate = true;
  room.templateId = id;
  if (!state.templates) state.templates = {};
  state.templates[id] = {
    name: templateName,
    imageDataUrl: room.imageDataUrl,
    sourceCoord: coordKeyStr,
    sourceMapId: mapId
  };
  return id;
}

/**
 * Check whether any room (other than the template source itself) uses this template.
 */
export function isTemplateInUse(state, templateId) {
  if (!templateId || !state.templates || !state.templates[templateId]) return false;
  const tmpl = state.templates[templateId];
  for (const mapId of Object.keys(state.maps)) {
    const rooms = state.maps[mapId].rooms;
    for (const key of Object.keys(rooms)) {
      const room = rooms[key];
      if (room.templateId === templateId) {
        // Skip the source room itself — it references its own template
        if (mapId === tmpl.sourceMapId && key === tmpl.sourceCoord) continue;
        return true;
      }
    }
  }
  return false;
}

/**
 * Return all rooms using a template, excluding the source room itself.
 */
export function getTemplateConsumerRooms(state, templateId) {
  if (!templateId || !state.templates || !state.templates[templateId]) return [];
  const tmpl = state.templates[templateId];
  const consumers = [];

  for (const mapId of Object.keys(state.maps)) {
    const map = state.maps[mapId];
    if (!map) continue;
    const rooms = map.rooms || {};
    for (const key of Object.keys(rooms)) {
      const room = rooms[key];
      if (room.templateId !== templateId) continue;
      if (mapId === tmpl.sourceMapId && key === tmpl.sourceCoord) continue;
      consumers.push({
        mapId,
        mapName: map.name || mapId,
        coord: key,
        x: room.x,
        y: room.y,
        roomId: room.roomId ?? null,
        title: room.title || ''
      });
    }
  }

  consumers.sort((a, b) =>
    a.mapName.localeCompare(b.mapName) ||
    a.y - b.y ||
    a.x - b.x ||
    a.coord.localeCompare(b.coord)
  );

  return consumers;
}

/**
 * Unmark a room as a template and remove it from the templates collection.
 * Returns true on success, false if the room wasn't a template.
 */
export function unmarkRoomAsTemplate(state, mapId, coordKeyStr) {
  const map = state.maps[mapId];
  if (!map) return false;
  const room = map.rooms[coordKeyStr];
  if (!room || !room.isTemplate) return false;

  const templateId = room.templateId;
  room.isTemplate = false;
  room.templateId = null;
  if (templateId && state.templates) {
    delete state.templates[templateId];
  }
  return true;
}

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
