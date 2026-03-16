import { describe, it, expect } from 'vitest';
import {
  coordKey,
  parseCoord,
  coordDisplay,
  makeRoom,
  generateId,
  buildDescription,
  buildAscDescription,
  buildLeaveFunction,
  parseLeaveFunction,
  sanitizeFilename,
  roomOverallType,
  ASC_DIRECTIONS,
  assignRoomIdsSync
} from './logic.js';

// ─── coordKey ───────────────────────────────────────────
describe('coordKey', () => {
  it('joins x,y with a comma', () => {
    expect(coordKey(3, 5)).toBe('3,5');
  });

  it('handles negative coordinates', () => {
    expect(coordKey(-1, -2)).toBe('-1,-2');
  });

  it('handles zero', () => {
    expect(coordKey(0, 0)).toBe('0,0');
  });
});

// ─── parseCoord ─────────────────────────────────────────
describe('parseCoord', () => {
  it('parses "3,5" into { x: 3, y: 5 }', () => {
    expect(parseCoord('3,5')).toEqual({ x: 3, y: 5 });
  });

  it('parses negative coordinates', () => {
    expect(parseCoord('-1,-2')).toEqual({ x: -1, y: -2 });
  });

  it('round-trips with coordKey', () => {
    const key = coordKey(7, -3);
    expect(parseCoord(key)).toEqual({ x: 7, y: -3 });
  });
});

// ─── coordDisplay ───────────────────────────────────────
describe('coordDisplay', () => {
  it('formats as XxY', () => {
    expect(coordDisplay(2, 3)).toBe('2x3');
  });

  it('handles negatives', () => {
    expect(coordDisplay(-1, 0)).toBe('-1x0');
  });
});

// ─── makeRoom ───────────────────────────────────────────
describe('makeRoom', () => {
  it('creates a room with correct defaults', () => {
    const room = makeRoom(4, 7);
    expect(room).toEqual({
      x: 4,
      y: 7,
      roomId: null,
      title: '',
      notes: '',
      imageDataUrl: null,
      imageFilename: null,
      isTemplate: false,
      templateId: null,
      complete: false
    });
  });
});

// ─── generateId ─────────────────────────────────────────
describe('generateId', () => {
  it('returns a non-empty string', () => {
    const id = generateId();
    expect(typeof id).toBe('string');
    expect(id.length).toBeGreaterThan(0);
  });

  it('generates unique IDs', () => {
    const ids = new Set(Array.from({ length: 100 }, () => generateId()));
    expect(ids.size).toBe(100);
  });
});

// ─── buildDescription ───────────────────────────────────
describe('buildDescription', () => {
  it('uses default format when none provided', () => {
    expect(buildDescription('Map1', 'Entrance', 2, 3)).toBe(
      'Map1 - Entrance 2x3'
    );
  });

  it('applies custom format', () => {
    expect(
      buildDescription('Map1', 'Entrance', 2, 3, '{TITLE} @ {COORDS}')
    ).toBe('Entrance @ 2x3');
  });

  it('replaces {X} and {Y} individually', () => {
    expect(
      buildDescription('M', 'T', 5, 8, 'x={X} y={Y}')
    ).toBe('x=5 y=8');
  });

  it('handles repeated tokens', () => {
    expect(
      buildDescription('M', 'T', 1, 2, '{MAP}-{MAP}')
    ).toBe('M-M');
  });
});

// ─── buildAscDescription ────────────────────────────────
describe('buildAscDescription', () => {
  it('wraps buildDescription in a comment', () => {
    expect(buildAscDescription('Map1', 'Room', 0, 0)).toBe(
      '// Map1 - Room 0x0'
    );
  });
});

// ─── buildLeaveFunction ─────────────────────────────────
describe('buildLeaveFunction', () => {
  it('builds a room_LeaveTop function', () => {
    const fn = buildLeaveFunction('Top', 5);
    expect(fn).toBe(
      'function room_LeaveTop()\n{\n  player.ChangeRoomAutoPosition(5);\n}'
    );
  });

  it('builds functions for each direction', () => {
    for (const dir of ['Top', 'Bottom', 'Left', 'Right']) {
      const fn = buildLeaveFunction(dir, 10);
      expect(fn).toContain(`room_Leave${dir}`);
      expect(fn).toContain('ChangeRoomAutoPosition(10)');
    }
  });
});

// ─── parseLeaveFunction ─────────────────────────────────
describe('parseLeaveFunction', () => {
  it('returns null when function not found', () => {
    expect(parseLeaveFunction('some code', 'Top')).toBeNull();
  });

  it('parses a simple leave function', () => {
    const content = buildLeaveFunction('Top', 5);
    const result = parseLeaveFunction(content, 'Top');
    expect(result).not.toBeNull();
    expect(result.isSimple).toBe(true);
    expect(result.start).toBe(0);
    expect(result.end).toBe(content.length);
    expect(result.fullText).toBe(content);
  });

  it('detects an empty body as simple', () => {
    const content = 'function room_LeaveBottom()\n{\n}';
    const result = parseLeaveFunction(content, 'Bottom');
    expect(result).not.toBeNull();
    expect(result.isSimple).toBe(true);
  });

  it('detects complex logic as not simple', () => {
    const content =
      'function room_LeaveLeft()\n{\n  if (hasKey) {\n    player.ChangeRoomAutoPosition(3);\n  }\n}';
    const result = parseLeaveFunction(content, 'Left');
    expect(result).not.toBeNull();
    expect(result.isSimple).toBe(false);
  });

  it('finds the function among other code', () => {
    const prefix = '// some comment\nint x = 5;\n\n';
    const func = buildLeaveFunction('Right', 7);
    const suffix = '\n\n// more code';
    const content = prefix + func + suffix;
    const result = parseLeaveFunction(content, 'Right');
    expect(result).not.toBeNull();
    expect(result.start).toBe(prefix.length);
    expect(result.end).toBe(prefix.length + func.length);
    expect(result.isSimple).toBe(true);
  });

  it('handles unmatched braces gracefully', () => {
    const content = 'function room_LeaveTop()\n{';
    const result = parseLeaveFunction(content, 'Top');
    expect(result).toBeNull();
  });

  it('ignores comment-only body lines', () => {
    const content =
      'function room_LeaveTop()\n{\n  // just a comment\n  player.ChangeRoomAutoPosition(2);\n}';
    const result = parseLeaveFunction(content, 'Top');
    expect(result).not.toBeNull();
    expect(result.isSimple).toBe(true);
  });
});

// ─── sanitizeFilename ───────────────────────────────────
describe('sanitizeFilename', () => {
  it('removes special characters', () => {
    expect(sanitizeFilename('Hello!@#World')).toBe('HelloWorld');
  });

  it('replaces spaces with underscores', () => {
    expect(sanitizeFilename('My Room Title')).toBe('My_Room_Title');
  });

  it('truncates at 50 characters', () => {
    const long = 'a'.repeat(60);
    expect(sanitizeFilename(long)).toHaveLength(50);
  });

  it('keeps allowed characters', () => {
    expect(sanitizeFilename('Room_1-test')).toBe('Room_1-test');
  });

  it('handles empty string', () => {
    expect(sanitizeFilename('')).toBe('');
  });

  it('collapses multiple spaces', () => {
    expect(sanitizeFilename('a   b')).toBe('a_b');
  });
});

// ─── roomOverallType ────────────────────────────────────
describe('roomOverallType', () => {
  it('returns "new" when .asc file was created', () => {
    const changes = [{ type: 'new', detail: 'Created new .asc file' }];
    expect(roomOverallType(changes)).toBe('new');
  });

  it('returns "update" when there are updates', () => {
    const changes = [{ type: 'update', detail: 'Updated description comment' }];
    expect(roomOverallType(changes)).toBe('update');
  });

  it('returns "update" when there are new (non-file-creation) changes', () => {
    const changes = [{ type: 'new', detail: 'Added room_LeaveTop() → Room 5' }];
    expect(roomOverallType(changes)).toBe('update');
  });

  it('returns "skip" when all changes are skips', () => {
    const changes = [
      { type: 'skip', detail: 'Room 1 already up to date' },
      { type: 'skip', detail: 'image.png already up to date' }
    ];
    expect(roomOverallType(changes)).toBe('skip');
  });

  it('returns "skip" for empty changes', () => {
    expect(roomOverallType([])).toBe('skip');
  });

  it('"new" file takes precedence over updates', () => {
    const changes = [
      { type: 'new', detail: 'Created new .asc file' },
      { type: 'update', detail: 'Updated description' }
    ];
    expect(roomOverallType(changes)).toBe('new');
  });
});

// ─── ASC_DIRECTIONS ─────────────────────────────────────
describe('ASC_DIRECTIONS', () => {
  it('has four directions', () => {
    expect(ASC_DIRECTIONS).toHaveLength(4);
  });

  it('covers Top, Bottom, Left, Right', () => {
    const names = ASC_DIRECTIONS.map((d) => d.name);
    expect(names).toEqual(['Top', 'Bottom', 'Left', 'Right']);
  });

  it('has correct deltas', () => {
    const byName = Object.fromEntries(ASC_DIRECTIONS.map((d) => [d.name, d]));
    expect(byName.Top).toEqual({ name: 'Top', dx: 0, dy: 1, crmEventIndex: 3 });
    expect(byName.Bottom).toEqual({ name: 'Bottom', dx: 0, dy: -1, crmEventIndex: 2 });
    expect(byName.Left).toEqual({ name: 'Left', dx: -1, dy: 0, crmEventIndex: 0 });
    expect(byName.Right).toEqual({ name: 'Right', dx: 1, dy: 0, crmEventIndex: 1 });
  });
});

// ─── assignRoomIdsSync ──────────────────────────────────
describe('assignRoomIdsSync', () => {
  function makeState(maps) {
    return { maps, activeMapId: Object.keys(maps)[0] || null, templates: {}, settings: {} };
  }

  it('assigns IDs starting from 1', () => {
    const state = makeState({
      m1: {
        name: 'Map1',
        rooms: {
          '0,0': makeRoom(0, 0),
          '1,0': makeRoom(1, 0)
        }
      }
    });
    const assignments = assignRoomIdsSync(state, []);
    expect(assignments).toHaveLength(2);
    expect(assignments[0].roomId).toBe(1);
    expect(assignments[1].roomId).toBe(2);
    expect(state.maps.m1.rooms['0,0'].roomId).toBe(1);
    expect(state.maps.m1.rooms['1,0'].roomId).toBe(2);
  });

  it('skips already-used IDs', () => {
    const room1 = makeRoom(0, 0);
    room1.roomId = 1;
    const state = makeState({
      m1: {
        name: 'Map1',
        rooms: {
          '0,0': room1,
          '1,0': makeRoom(1, 0)
        }
      }
    });
    const assignments = assignRoomIdsSync(state, []);
    expect(assignments).toHaveLength(1);
    expect(assignments[0].roomId).toBe(2);
  });

  it('skips IDs used by existing files', () => {
    const state = makeState({
      m1: {
        name: 'Map1',
        rooms: { '0,0': makeRoom(0, 0) }
      }
    });
    const assignments = assignRoomIdsSync(state, [1, 2, 3]);
    expect(assignments).toHaveLength(1);
    expect(assignments[0].roomId).toBe(4);
  });

  it('skips complete rooms', () => {
    const completeRoom = makeRoom(0, 0);
    completeRoom.complete = true;
    const state = makeState({
      m1: {
        name: 'Map1',
        rooms: {
          '0,0': completeRoom,
          '1,0': makeRoom(1, 0)
        }
      }
    });
    const assignments = assignRoomIdsSync(state, []);
    expect(assignments).toHaveLength(1);
    expect(assignments[0].roomId).toBe(1);
    expect(completeRoom.roomId).toBeNull();
  });

  it('works across multiple maps', () => {
    const state = makeState({
      m1: {
        name: 'Map1',
        rooms: { '0,0': makeRoom(0, 0) }
      },
      m2: {
        name: 'Map2',
        rooms: { '0,0': makeRoom(0, 0) }
      }
    });
    const assignments = assignRoomIdsSync(state, []);
    expect(assignments).toHaveLength(2);
    const ids = assignments.map((a) => a.roomId);
    expect(ids).toEqual([1, 2]);
  });

  it('returns empty array when all rooms are assigned', () => {
    const room = makeRoom(0, 0);
    room.roomId = 5;
    const state = makeState({
      m1: { name: 'Map1', rooms: { '0,0': room } }
    });
    const assignments = assignRoomIdsSync(state, []);
    expect(assignments).toHaveLength(0);
  });

  it('skips both map-used and file-used IDs', () => {
    const room1 = makeRoom(0, 0);
    room1.roomId = 1;
    const state = makeState({
      m1: {
        name: 'Map1',
        rooms: {
          '0,0': room1,
          '1,0': makeRoom(1, 0)
        }
      }
    });
    // File has room2.asc, map has room 1 → next available is 3
    const assignments = assignRoomIdsSync(state, [2]);
    expect(assignments).toHaveLength(1);
    expect(assignments[0].roomId).toBe(3);
  });
});
