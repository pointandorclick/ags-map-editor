# Changelog

## v1.3.1
- fix: prevent template source rooms from corrupting their own .crm files during generation
- feat: add blocked adge toggle to not create room_Leave for blocked edges 

## v1.3.0
- feat: Images from map screens are automatically embedded into room.crm files
- fix: harden backend validation, debounce saves, and eliminate innerHTML XSS vectors
- test: add coverage for CRM event registration, LZSS edge cases, and error status handling
- feat: add context aware template mark/unmark and indicator
- style: make UI less intrusive and change feature colour to pink
- style: add light/dark theme switcher

## v1.2.5
- fix: right click context menu for create template more reliable

## v1.2.4
- fix: re-instate drag and drop of backgrounds

## v1.2.3
- fix: false-positive .crm warning and list affected rooms
- fix: template file copy — always force overwrite, reset on roomId change, and fix new room detection
- fix: save base room and base room allocation process

## v1.2.2
- feat: Use a base room, provided by the user, to create future rooms.

## v1.1.1
- fix: dynamically scan project for room IDs and assign new IDs on generate
- fix: update GitHub Actions to v5 for Node.js 24 support

## v1.1.0
- feat: allow change of order of rooms via drag and drop

## v1.0.2
- fix: an empty room_Leave function was not updating
- feat: global settings added to format AGS room descriptions
- feat: Fix inverted Y-axis in ASC_DIRECTIONS for room_Leave generation

## v1.0.1
- fix: Add linux and windows distribution files

## v1.0.0

- Initial release
