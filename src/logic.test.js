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
  assignRoomIdsSync,
  resolveTemplateSourceRoomId,
  markRoomAsTemplate,
  isTemplateInUse,
  getTemplateConsumerRooms,
  unmarkRoomAsTemplate,
  removeSimpleLeaveFunction
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
      lastAppliedTemplateId: null,
      complete: false,
      blockedEdges: {}
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
  it('returns "new" when room ID was assigned', () => {
    const changes = [{ type: 'new', detail: 'Assigned Room ID 34' }];
    expect(roomOverallType(changes)).toBe('new');
  });

  it('returns "new" when room ID assigned with template copies', () => {
    const changes = [
      { type: 'new', detail: 'Assigned Room ID 34' },
      { type: 'new', detail: 'Template: copied:room34.crm' },
      { type: 'update', detail: 'Updated description comment' }
    ];
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

  it('"new" room ID takes precedence over updates', () => {
    const changes = [
      { type: 'new', detail: 'Assigned Room ID 10' },
      { type: 'update', detail: 'Updated description' }
    ];
    expect(roomOverallType(changes)).toBe('new');
  });

  it('returns "error" when any change has type error', () => {
    const changes = [
      { type: 'update', detail: 'Updated description' },
      { type: 'error', detail: 'Embed background failed' }
    ];
    expect(roomOverallType(changes)).toBe('error');
  });

  it('"new" room ID takes precedence over error', () => {
    const changes = [
      { type: 'new', detail: 'Assigned Room ID 5' },
      { type: 'error', detail: 'Dimension mismatch' }
    ];
    expect(roomOverallType(changes)).toBe('new');
  });

  it('"error" takes precedence over "update"', () => {
    const changes = [
      { type: 'new', detail: 'Created new .asc file' },
      { type: 'update', detail: 'Updated handler' },
      { type: 'error', detail: 'CRM write failed' }
    ];
    expect(roomOverallType(changes)).toBe('error');
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

// ─── resolveTemplateSourceRoomId ────────────────────────
describe('resolveTemplateSourceRoomId', () => {
  function makeState(maps, templates) {
    return { maps, activeMapId: Object.keys(maps)[0] || null, templates: templates || {}, settings: {} };
  }

  it('resolves a valid template to its source room ID', () => {
    const room = makeRoom(2, 3);
    room.roomId = 42;
    const state = makeState(
      { m1: { name: 'Map1', rooms: { '2,3': room } } },
      { t1: { sourceCoord: '2,3', sourceMapId: 'm1' } }
    );
    expect(resolveTemplateSourceRoomId(state, 't1')).toBe(42);
  });

  it('returns null for missing template', () => {
    const state = makeState({ m1: { name: 'Map1', rooms: {} } }, {});
    expect(resolveTemplateSourceRoomId(state, 'nonexistent')).toBeNull();
  });

  it('returns null for missing map', () => {
    const state = makeState(
      { m1: { name: 'Map1', rooms: {} } },
      { t1: { sourceCoord: '0,0', sourceMapId: 'missing' } }
    );
    expect(resolveTemplateSourceRoomId(state, 't1')).toBeNull();
  });

  it('returns null for missing room', () => {
    const state = makeState(
      { m1: { name: 'Map1', rooms: {} } },
      { t1: { sourceCoord: '5,5', sourceMapId: 'm1' } }
    );
    expect(resolveTemplateSourceRoomId(state, 't1')).toBeNull();
  });

  it('returns null when room has null roomId', () => {
    const room = makeRoom(0, 0);
    // room.roomId is null by default
    const state = makeState(
      { m1: { name: 'Map1', rooms: { '0,0': room } } },
      { t1: { sourceCoord: '0,0', sourceMapId: 'm1' } }
    );
    expect(resolveTemplateSourceRoomId(state, 't1')).toBeNull();
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

// ─── markRoomAsTemplate ────────────────────────────────
describe('markRoomAsTemplate', () => {
  function makeState(maps, templates) {
    return { maps, activeMapId: Object.keys(maps)[0] || null, templates: templates || {}, settings: {} };
  }

  it('marks a room with an image as a template and adds to templates', () => {
    const room = makeRoom(1, 2);
    room.imageDataUrl = 'data:image/png;base64,abc123';
    room.title = 'Hallway';
    const state = makeState({ m1: { name: 'Map1', rooms: { '1,2': room } } });

    const templateId = markRoomAsTemplate(state, 'm1', '1,2', 'My Template');

    expect(templateId).not.toBeNull();
    expect(room.isTemplate).toBe(true);
    expect(room.templateId).toBe(templateId);
    expect(state.templates[templateId]).toEqual({
      name: 'My Template',
      imageDataUrl: 'data:image/png;base64,abc123',
      sourceCoord: '1,2',
      sourceMapId: 'm1'
    });
  });

  it('returns null when the room has no image', () => {
    const room = makeRoom(0, 0);
    const state = makeState({ m1: { name: 'Map1', rooms: { '0,0': room } } });

    const templateId = markRoomAsTemplate(state, 'm1', '0,0', 'Empty');

    expect(templateId).toBeNull();
    expect(room.isTemplate).toBe(false);
    expect(Object.keys(state.templates)).toHaveLength(0);
  });

  it('returns null for an invalid map ID', () => {
    const state = makeState({ m1: { name: 'Map1', rooms: {} } });
    expect(markRoomAsTemplate(state, 'bad-id', '0,0', 'Nope')).toBeNull();
  });

  it('returns null for a non-existent room coord', () => {
    const state = makeState({ m1: { name: 'Map1', rooms: { '0,0': makeRoom(0, 0) } } });
    expect(markRoomAsTemplate(state, 'm1', '5,5', 'Nope')).toBeNull();
  });

  it('initialises templates object when missing', () => {
    const room = makeRoom(0, 0);
    room.imageDataUrl = 'data:image/png;base64,xyz';
    const state = { maps: { m1: { name: 'Map1', rooms: { '0,0': room } } }, activeMapId: 'm1' };

    const templateId = markRoomAsTemplate(state, 'm1', '0,0', 'First');

    expect(templateId).not.toBeNull();
    expect(state.templates[templateId].name).toBe('First');
  });
});

// ─── isTemplateInUse ───────────────────────────────────
describe('isTemplateInUse', () => {
  function makeState(maps, templates) {
    return { maps, activeMapId: Object.keys(maps)[0] || null, templates: templates || {}, settings: {} };
  }

  it('returns false when no other room references the template', () => {
    const room = makeRoom(0, 0);
    room.isTemplate = true;
    room.templateId = 't1';
    room.imageDataUrl = 'data:image/png;base64,abc';
    const state = makeState(
      { m1: { name: 'Map1', rooms: { '0,0': room } } },
      { t1: { name: 'Tmpl', imageDataUrl: 'x', sourceCoord: '0,0', sourceMapId: 'm1' } }
    );
    expect(isTemplateInUse(state, 't1')).toBe(false);
  });

  it('returns true when another room in the same map uses the template', () => {
    const src = makeRoom(0, 0);
    src.isTemplate = true;
    src.templateId = 't1';
    src.imageDataUrl = 'data:image/png;base64,abc';
    const consumer = makeRoom(1, 0);
    consumer.templateId = 't1';
    const state = makeState(
      { m1: { name: 'Map1', rooms: { '0,0': src, '1,0': consumer } } },
      { t1: { name: 'Tmpl', imageDataUrl: 'x', sourceCoord: '0,0', sourceMapId: 'm1' } }
    );
    expect(isTemplateInUse(state, 't1')).toBe(true);
  });

  it('returns true when a room in a different map uses the template', () => {
    const src = makeRoom(0, 0);
    src.isTemplate = true;
    src.templateId = 't1';
    src.imageDataUrl = 'data:image/png;base64,abc';
    const consumer = makeRoom(0, 0);
    consumer.templateId = 't1';
    const state = makeState(
      {
        m1: { name: 'Map1', rooms: { '0,0': src } },
        m2: { name: 'Map2', rooms: { '0,0': consumer } }
      },
      { t1: { name: 'Tmpl', imageDataUrl: 'x', sourceCoord: '0,0', sourceMapId: 'm1' } }
    );
    expect(isTemplateInUse(state, 't1')).toBe(true);
  });

  it('returns false for a non-existent template ID', () => {
    const state = makeState({ m1: { name: 'Map1', rooms: {} } });
    expect(isTemplateInUse(state, 'nope')).toBe(false);
  });

  it('returns false when templates object is missing', () => {
    const state = { maps: { m1: { name: 'Map1', rooms: {} } }, activeMapId: 'm1' };
    expect(isTemplateInUse(state, 't1')).toBe(false);
  });
});

// ─── getTemplateConsumerRooms ──────────────────────────
describe('getTemplateConsumerRooms', () => {
  function makeState(maps, templates) {
    return { maps, activeMapId: Object.keys(maps)[0] || null, templates: templates || {}, settings: {} };
  }

  it('returns template consumer rooms and excludes the source room', () => {
    const src = makeRoom(0, 0);
    src.isTemplate = true;
    src.templateId = 't1';
    const consumer = makeRoom(1, 2);
    consumer.templateId = 't1';
    consumer.roomId = 12;
    consumer.title = 'Hallway';
    const state = makeState(
      { m1: { name: 'Map1', rooms: { '0,0': src, '1,2': consumer } } },
      { t1: { name: 'Tmpl', imageDataUrl: 'x', sourceCoord: '0,0', sourceMapId: 'm1' } }
    );

    expect(getTemplateConsumerRooms(state, 't1')).toEqual([
      {
        mapId: 'm1',
        mapName: 'Map1',
        coord: '1,2',
        x: 1,
        y: 2,
        roomId: 12,
        title: 'Hallway'
      }
    ]);
  });

  it('returns an empty array for missing templates', () => {
    const state = makeState({ m1: { name: 'Map1', rooms: {} } });
    expect(getTemplateConsumerRooms(state, 'missing')).toEqual([]);
  });

  it('sorts consumers by map name and coordinates', () => {
    const src = makeRoom(0, 0);
    src.isTemplate = true;
    src.templateId = 't1';

    const a = makeRoom(3, 1);
    a.templateId = 't1';
    a.title = 'A';

    const b = makeRoom(1, 5);
    b.templateId = 't1';
    b.title = 'B';

    const c = makeRoom(0, 2);
    c.templateId = 't1';
    c.title = 'C';

    const state = makeState(
      {
        zmap: { name: 'Zoo', rooms: { '3,1': a } },
        amap: { name: 'Alpha', rooms: { '0,0': src, '1,5': b, '0,2': c } }
      },
      { t1: { name: 'Tmpl', imageDataUrl: 'x', sourceCoord: '0,0', sourceMapId: 'amap' } }
    );

    expect(getTemplateConsumerRooms(state, 't1').map((room) => room.coord)).toEqual(['0,2', '1,5', '3,1']);
  });
});

// ─── unmarkRoomAsTemplate ──────────────────────────────
describe('unmarkRoomAsTemplate', () => {
  function makeState(maps, templates) {
    return { maps, activeMapId: Object.keys(maps)[0] || null, templates: templates || {}, settings: {} };
  }

  it('clears isTemplate, templateId on the room and removes the template entry', () => {
    const room = makeRoom(0, 0);
    room.isTemplate = true;
    room.templateId = 't1';
    room.imageDataUrl = 'data:image/png;base64,abc';
    const state = makeState(
      { m1: { name: 'Map1', rooms: { '0,0': room } } },
      { t1: { name: 'Tmpl', imageDataUrl: 'x', sourceCoord: '0,0', sourceMapId: 'm1' } }
    );

    const result = unmarkRoomAsTemplate(state, 'm1', '0,0');

    expect(result).toBe(true);
    expect(room.isTemplate).toBe(false);
    expect(room.templateId).toBeNull();
    expect(state.templates).not.toHaveProperty('t1');
  });

  it('returns false for a room that is not a template', () => {
    const room = makeRoom(0, 0);
    room.imageDataUrl = 'data:image/png;base64,abc';
    const state = makeState({ m1: { name: 'Map1', rooms: { '0,0': room } } });

    expect(unmarkRoomAsTemplate(state, 'm1', '0,0')).toBe(false);
    expect(room.isTemplate).toBe(false);
  });

  it('returns false for an invalid map ID', () => {
    const state = makeState({ m1: { name: 'Map1', rooms: {} } });
    expect(unmarkRoomAsTemplate(state, 'bad', '0,0')).toBe(false);
  });

  it('returns false for a non-existent room coord', () => {
    const state = makeState({ m1: { name: 'Map1', rooms: { '0,0': makeRoom(0, 0) } } });
    expect(unmarkRoomAsTemplate(state, 'm1', '5,5')).toBe(false);
  });

  it('preserves the room image after unmarking', () => {
    const room = makeRoom(0, 0);
    room.isTemplate = true;
    room.templateId = 't1';
    room.imageDataUrl = 'data:image/png;base64,abc';
    const state = makeState(
      { m1: { name: 'Map1', rooms: { '0,0': room } } },
      { t1: { name: 'Tmpl', imageDataUrl: 'x', sourceCoord: '0,0', sourceMapId: 'm1' } }
    );

    unmarkRoomAsTemplate(state, 'm1', '0,0');

    expect(room.imageDataUrl).toBe('data:image/png;base64,abc');
  });
});

// ─── removeSimpleLeaveFunction ─────────────────────────
describe('removeSimpleLeaveFunction', () => {
  it('removes a simple leave function', () => {
    const content = '// desc\n\nfunction room_LeaveRight()\n{\n  player.ChangeRoomAutoPosition(5);\n}\n';
    const result = removeSimpleLeaveFunction(content, 'Right');
    expect(result.removed).toBe(true);
    expect(result.content).not.toContain('room_LeaveRight');
    expect(result.content).toContain('// desc');
  });

  it('leaves complex function untouched', () => {
    const content = 'function room_LeaveRight()\n{\n  if (hasKey) player.ChangeRoomAutoPosition(5);\n}\n';
    const result = removeSimpleLeaveFunction(content, 'Right');
    expect(result.removed).toBe(false);
    expect(result.content).toContain('room_LeaveRight');
  });

  it('returns unchanged when function does not exist', () => {
    const content = '// just a comment\n';
    const result = removeSimpleLeaveFunction(content, 'Top');
    expect(result.removed).toBe(false);
    expect(result.content).toBe(content);
  });

  it('only removes the targeted direction', () => {
    const content = [
      '// desc',
      '',
      'function room_LeaveLeft()',
      '{',
      '  player.ChangeRoomAutoPosition(3);',
      '}',
      '',
      'function room_LeaveRight()',
      '{',
      '  player.ChangeRoomAutoPosition(5);',
      '}',
      ''
    ].join('\n');
    const result = removeSimpleLeaveFunction(content, 'Right');
    expect(result.removed).toBe(true);
    expect(result.content).toContain('room_LeaveLeft');
    expect(result.content).not.toContain('room_LeaveRight');
  });
});
