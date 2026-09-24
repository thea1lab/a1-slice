---
version: alpha
name: A1-Slice
description: A dark desktop window for working on one video at a time. Charcoal surfaces, cream text, and a single orange for the action the person should take next. Serif headlines, a plain sans for everything else, and a thin sunset stripe along the bottom of tool screens. Home is a centered choice of tools. Each tool explains the work in plain language and keeps what it saves next to the video.

colors:
  primary: "#fa520f"
  primary-deep: "#cc3a05"
  on-primary: "#ffffff"
  sunshine-500: "#ffb83e"
  sunshine-800: "#ff8105"
  yellow-saturated: "#ffd900"
  cream: "#fff8e0"
  cream-text: "#f4f1ea"
  cream-muted: "#d9d3c5"
  beige-deep: "#e6d5a8"
  ink: "#1f1f1f"
  canvas: "#161616"
  surface: "#1c1c1e"
  surface-input: "#141414"
  surface-selected: "#241c18"
  hairline: "rgba(255, 255, 255, 0.08)"
  hairline-strong: "#3a3a3a"
  hairline-hover: "#6a6a6a"
  slate: "#4a4a4a"
  steel: "#6a6a6a"
  stone: "#8a8a8a"
  muted: "#a8a8a8"
  danger-border: "#7a3b32"
  danger-fill: "#3a201c"
  link: "#fa520f"

typography:
  display:
    fontFamily: Noto Serif
    fontSize: 52px
    fontWeight: 450
    lineHeight: 1.15
    letterSpacing: -0.4px
  display-home:
    fontFamily: Noto Serif
    fontSize: 56px
    fontWeight: 450
    lineHeight: 1.15
    letterSpacing: -0.4px
  heading-3:
    fontFamily: Inter
    fontSize: 22px
    fontWeight: 500
    lineHeight: 1.30
  heading-5:
    fontFamily: Inter
    fontSize: 18px
    fontWeight: 500
    lineHeight: 1.40
  subtitle:
    fontFamily: Inter
    fontSize: 18px
    fontWeight: 400
    lineHeight: 1.50
  body-md:
    fontFamily: Inter
    fontSize: 16px
    fontWeight: 400
    lineHeight: 1.55
  body-md-medium:
    fontFamily: Inter
    fontSize: 16px
    fontWeight: 500
    lineHeight: 1.55
  body-sm:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: 400
    lineHeight: 1.50
  body-sm-medium:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: 500
    lineHeight: 1.50
  caption:
    fontFamily: Inter
    fontSize: 13px
    fontWeight: 400
    lineHeight: 1.40
  micro-uppercase:
    fontFamily: Inter
    fontSize: 11px
    fontWeight: 600
    lineHeight: 1.40
    letterSpacing: 1px
  button-md:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: 500
    lineHeight: 1.30
  button-lg:
    fontFamily: Inter
    fontSize: 16px
    fontWeight: 500
    lineHeight: 1.30

rounded:
  sm: 6px
  md: 8px
  lg: 12px
  full: 9999px

spacing:
  xxs: 4px
  xs: 8px
  sm: 12px
  md: 16px
  lg: 20px
  xl: 24px
  xxl: 28px
  xxxl: 40px
  section: 48px

components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
    typography: "{typography.button-md}"
    rounded: "{rounded.md}"
    padding: "10px 20px"
  button-primary-pressed:
    backgroundColor: "{colors.primary-deep}"
    textColor: "{colors.on-primary}"
  button-primary-disabled:
    backgroundColor: "{colors.slate}"
    textColor: "{colors.stone}"
  button-secondary:
    backgroundColor: "transparent"
    textColor: "{colors.cream-text}"
    typography: "{typography.button-md}"
    rounded: "{rounded.md}"
    padding: "10px 20px"
    border: "1px solid {colors.slate}"
  button-danger:
    backgroundColor: "transparent"
    textColor: "{colors.cream-text}"
    typography: "{typography.button-md}"
    rounded: "{rounded.md}"
    padding: "10px 20px"
    border: "1px solid {colors.danger-border}"
  button-lg:
    typography: "{typography.button-lg}"
    padding: "12px 22px"
    height: 48px
  home-back:
    backgroundColor: "transparent"
    textColor: "{colors.cream}"
    typography: "{typography.body-md-medium}"
    rounded: "{rounded.md}"
    padding: "0 12px 0 8px"
    height: 36px
  card:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.cream-text}"
    rounded: "{rounded.lg}"
    padding: "{spacing.xl}"
    border: "1px solid {colors.hairline}"
  home-tile:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.cream-text}"
    rounded: "{rounded.lg}"
    padding: "36px 28px"
    border: "1px solid {colors.hairline}"
  choice:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.cream-text}"
    rounded: "{rounded.lg}"
    padding: "16px 18px"
    border: "1px solid {colors.hairline}"
  choice-selected:
    backgroundColor: "{colors.surface-selected}"
    textColor: "{colors.cream}"
    rounded: "{rounded.lg}"
    padding: "16px 18px"
    border: "2px solid {colors.primary}"
  option:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.cream-text}"
    typography: "{typography.body-md-medium}"
    rounded: "{rounded.md}"
    padding: "10px 16px"
    height: 44px
    border: "1px solid {colors.hairline-strong}"
  option-selected:
    backgroundColor: "{colors.surface-selected}"
    textColor: "{colors.cream}"
    rounded: "{rounded.md}"
    border: "1px solid {colors.primary}"
  text-input:
    backgroundColor: "{colors.surface-input}"
    textColor: "{colors.cream-text}"
    typography: "{typography.body-md}"
    rounded: "{rounded.md}"
    padding: "12px 14px"
    border: "1px solid {colors.hairline-strong}"
  text-input-focused:
    backgroundColor: "{colors.surface-input}"
    textColor: "{colors.cream-text}"
    border: "1px solid {colors.primary}"
  icon-tile:
    backgroundColor: "{colors.cream}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    padding: "{spacing.md}"
    border: "1px solid {colors.beige-deep}"
  notice:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.cream-text}"
    rounded: "{rounded.lg}"
    padding: "{spacing.lg}"
    border: "1px solid {colors.hairline}"
  file-path:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.cream}"
    typography: "{typography.body-sm}"
    rounded: "{rounded.md}"
    padding: "12px 14px"
    border: "1px solid {colors.hairline}"
  sunset-stripe:
    backgroundColor: "{colors.primary}"
    rounded: "0"
    height: 4px
  titlebar:
    backgroundColor: "{colors.canvas}"
    textColor: "{colors.cream}"
    typography: "{typography.body-sm-medium}"
    padding: "0 {spacing.md}"
    height: 48px
---

## Overview

This is the visual system for a local desktop app that works on one video at a time. The window is dark. Headlines are a warm serif. Labels, buttons, and explanations are a plain sans. Orange is reserved for the action to take next and for a selected choice. A four-pixel sunset stripe closes tool screens.

Home asks what to do, then offers the tools as content-sized cards in a centered pair of columns. Opening a tool does not ask for a file yet. The tool screen explains the work, puts Home at the top left, and only then asks for a video. Saved work stays in files next to that video.

**Key characteristics:**

- Charcoal window (`{colors.canvas}`) with slightly lighter cards (`{colors.surface}`)
- Cream headlines (`{colors.cream}`) and warmer body text (`{colors.cream-text}`)
- One orange (`{colors.primary}`) for the primary button and the selected choice
- Noto Serif for display headlines, Inter for the rest of the interface
- `{rounded.md}` (8px) buttons and inputs, `{rounded.lg}` (12px) cards
- A thin sunset stripe at the bottom of tool screens
- Short choices are buttons in a row, so the control never covers itself

## Colors

### Action

- **Orange** (`{colors.primary}`): the primary button, a selected border, a focused field, and the start of the sunset stripe
- **Orange deep** (`{colors.primary-deep}`): the primary button while the pointer is down
- **On orange** (`{colors.on-primary}`): label on a primary button

### Sunset stripe

These stops exist for the bottom stripe. They are not button colors.

- **Sunshine 800** (`{colors.sunshine-800}`)
- **Sunshine 500** (`{colors.sunshine-500}`)
- **Yellow** (`{colors.yellow-saturated}`)
- **Cream** (`{colors.cream}`), the last stop

The stripe runs `{colors.primary}` → `{colors.sunshine-800}` → `{colors.sunshine-500}` → `{colors.yellow-saturated}` → `{colors.cream}`.

### Text

- **Cream** (`{colors.cream}`): display headlines, section labels, the Home control, and selected option labels
- **Cream text** (`{colors.cream-text}`): body copy and button labels
- **Cream muted** (`{colors.cream-muted}`): a quieter warm tone when cream text is too strong
- **Muted** (`{colors.muted}`): help text, file names in the title bar, timestamps
- **Stone** (`{colors.stone}`): disabled labels
- **Ink** (`{colors.ink}`): text and icons that sit on a cream tile

### Surfaces

- **Canvas** (`{colors.canvas}`): the window background and the title bar
- **Surface** (`{colors.surface}`): cards, choices, and option buttons
- **Input** (`{colors.surface-input}`): text fields
- **Selected** (`{colors.surface-selected}`): the fill behind a selected choice or option
- **Hairline** (`{colors.hairline}`): card borders and quiet dividers
- **Hairline strong** (`{colors.hairline-strong}`): field and option borders
- **Slate** (`{colors.slate}`): the outline of a secondary button, and the fill of a disabled primary button

### Danger

- **Danger border** (`{colors.danger-border}`): cancel and other destructive outlines
- **Danger fill** (`{colors.danger-fill}`): the same button while the pointer is down

## Typography

### Font families

**Noto Serif** is the display face for the home headline and each tool title. Fallbacks: Georgia, Times New Roman, serif.

**Inter** is the face for explanations, buttons, fields, lists, and the title bar. Fallbacks: ui-sans-serif, system-ui, sans-serif.

File paths use Inter as well, with long paths allowed to wrap. A separate monospace face is not part of the interface.

### Hierarchy

| Token | Size | Weight | Line height | Letter spacing | Family | Use |
|---|---|---|---|---|---|---|
| `{typography.display-home}` | 56px | 450 | 1.15 | -0.4px | Noto Serif | Home headline. Shrinks toward 40px in a narrower window. |
| `{typography.display}` | 52px | 450 | 1.15 | -0.4px | Noto Serif | Tool screen title. Shrinks toward 40px. |
| `{typography.heading-3}` | 22px | 500 | 1.30 | 0 | Inter | Tool card titles on Home |
| `{typography.heading-5}` | 18px | 500 | 1.40 | 0 | Inter | Section labels inside a tool |
| `{typography.subtitle}` | 18px | 400 | 1.50 | 0 | Inter | The lead paragraph on a tool screen |
| `{typography.body-md}` | 16px | 400 | 1.55 | 0 | Inter | Explanations and field values |
| `{typography.body-md-medium}` | 16px | 500 | 1.55 | 0 | Inter | Home back label, choice titles |
| `{typography.body-sm}` | 14px | 400 | 1.50 | 0 | Inter | Card descriptions, help under a choice |
| `{typography.body-sm-medium}` | 14px | 500 | 1.50 | 0 | Inter | Standard button labels |
| `{typography.button-lg}` | 16px | 500 | 1.30 | 0 | Inter | The main button on a tool screen |
| `{typography.caption}` | 13px | 400 | 1.40 | 0 | Inter | Timestamps and small meta |
| `{typography.micro-uppercase}` | 11px | 600 | 1.40 | 1px | Inter | The small label above the home headline |

### Principles

- The serif is for the question or the tool name. Everything the person reads next is Inter.
- Body line height stays near 1.5 so a short explanation is easy to read.
- Display letter-spacing stays slightly tight. UI text stays at 0.
- Say what the control does in ordinary words. A setting that is rarely needed stays collapsed, with a sentence that says when to open it.

## Layout

### Spacing

- **Base unit:** 4px
- **Tokens:** `{spacing.xxs}` 4px · `{spacing.xs}` 8px · `{spacing.sm}` 12px · `{spacing.md}` 16px · `{spacing.lg}` 20px · `{spacing.xl}` 24px · `{spacing.xxl}` 28px · `{spacing.xxxl}` 40px · `{spacing.section}` 48px
- Tool screens stack sections with about `{spacing.xxl}` between them
- Home uses `{spacing.xxxl}` between the headline and the tool cards
- The window content inset is `{spacing.section}` on the sides when there is room

### Window

The default window is about 1320×860. Home content sits in the middle of that window, in a column no wider than about 920px. A tool screen uses a left-aligned sheet about 720px wide inside the same window, with room to scroll. Player screens (review, reframe, captions) can use the width of the window because the video is the work.

### Home

- A small uppercase label, then one serif headline, then one sentence
- A two-column grid of tool cards
- Each card holds a cream icon tile, a title, and one sentence, grouped at the top of the card
- Cards are as tall as their content. They do not stretch to fill empty window space

### Tool screen

- Title bar: Home at the left, then the tool name and the video file name when one is open
- The sheet explains the step, then offers the next action
- Recent videos, when shown, are an optional list under the choose-a-video action. A file with nothing saved shows only its name
- One primary button per step. Other actions are secondary

### Whitespace

Give the headline and the next action air. Help text sits close to the control it explains. Empty regions of the window stay the canvas color. Do not fill them with decoration.

## Elevation and depth

The window is flat. Separation comes from a one-pixel hairline and a small step in surface color.

| Level | Treatment | Use |
|---|---|---|
| 0 | Canvas `{colors.canvas}` | Window, title bar |
| 1 | Surface `{colors.surface}` plus `{colors.hairline}` | Cards, choices, notices |
| 2 | `{colors.surface-selected}` plus a `{colors.primary}` border | The selected choice |

Shadows are not part of this system. The sunset stripe is a color band, not a shadow.

## Shapes

| Token | Value | Use |
|---|---|---|
| `{rounded.md}` | 8px | Buttons, fields, the Home control, option buttons, icon tiles |
| `{rounded.lg}` | 12px | Cards, tool tiles, choices, notices, large icon tiles |
| `{rounded.full}` | 9999px | A slider thumb only |

Buttons are rounded rectangles. A fully round button is not used for actions.

Icon tiles are cream squares with ink-colored icons: a microphone, a clapper, overlapping frames, and a caption box for the four tools.

## Components

Pressed and selected states are specified. Hover only lightens a border or a quiet fill. It does not introduce a new color.

### Buttons

**`button-primary`** — the action to take next.

- Background `{colors.primary}`, text `{colors.on-primary}`, type `{typography.button-md}`, padding `10px 20px`, radius `{rounded.md}`
- Pressed: `{colors.primary-deep}`
- Disabled: background `{colors.slate}`, text `{colors.stone}`
- On a tool screen, use `button-lg`: 48px tall, `{typography.button-lg}`, padding `12px 22px`

**`button-secondary`** — an available action that is not the main one.

- Transparent background, text `{colors.cream-text}`, border `1px solid {colors.slate}`, same radius and padding as the primary button

**`button-danger`** — cancel, or something that throws work away.

- Transparent background, text `{colors.cream-text}`, border `1px solid {colors.danger-border}`
- Pressed fill: `{colors.danger-fill}`

**`home-back`** — the way back to Home, at the top left of every tool screen.

- Transparent background, cream text, a left chevron, label “Home”, radius `{rounded.md}`
- Pressed or hover fill: a faint white wash, about 6% white

### Cards and choices

**`card`** — a quiet panel for a block of content.

- Background `{colors.surface}`, radius `{rounded.lg}`, padding `{spacing.xl}`, border `1px solid {colors.hairline}`

**`home-tile`** — one tool on the home screen.

- Same surface and radius as a card, padding `36px 28px`, content centered
- Cream icon tile, title in `{typography.heading-3}` and `{colors.cream}`, one sentence in `{typography.body-sm}` and `{colors.muted}`

**`choice`** — a larger selectable row, such as a caption style.

- Background `{colors.surface}`, radius `{rounded.lg}`, padding `16px 18px`, border `1px solid {colors.hairline}`
- Title in `{colors.cream}`, help in `{colors.muted}`

**`choice-selected`** — the choice that is on.

- Background `{colors.surface-selected}`, border `2px solid {colors.primary}`

**`option`** and **`option-selected`** — a short set of answers, such as a spoken language.

- Laid out in a wrapping row with `{spacing.xs}` gaps
- Unselected: surface fill, `{colors.hairline-strong}` border, 44px tall, radius `{rounded.md}`
- Selected: `{colors.surface-selected}` fill, `{colors.primary}` border, `{colors.cream}` label
- Use this instead of a native dropdown whenever the list is short

### Fields

**`text-input`** — a text field or a number field.

- Background `{colors.surface-input}`, text `{colors.cream-text}`, border `1px solid {colors.hairline-strong}`, radius `{rounded.md}`, padding `12px 14px`

**`text-input-focused`** — the border becomes `{colors.primary}`.

A number that people rarely change sits in a collapsed group. The summary says when to open it. Each row names the setting in plain language and puts the field on the right.

### Icon tile

**`icon-tile`** — the cream square that holds a tool icon.

- Background `{colors.cream}`, icon `{colors.ink}`, border `1px solid {colors.beige-deep}`, radius `{rounded.md}`
- 40px in a compact row, 64px with `{rounded.lg}` on Home and at the top of a tool guide

### Notice and file path

**`notice`** — a callout when the video already has saved work.

- Background `{colors.surface}`, radius `{rounded.lg}`, padding `{spacing.lg}`, border `1px solid {colors.hairline}`
- One or two sentences, then the actions

**`file-path`** — the path of a file the app just wrote, shown so the person can find it.

- Background `{colors.surface}`, text `{colors.cream}`, padding `12px 14px`, radius `{rounded.md}`, border `1px solid {colors.hairline}`
- The path wraps. It is the full path, in the same folder as the video

### Chrome

**`titlebar`** — the top of the window.

- Height 48px, background `{colors.canvas}`, horizontal padding `{spacing.md}`
- On Home, the product name
- On a tool, `home-back`, then the tool name in cream and the file name in `{colors.muted}`
- No line under the bar

**`sunset-stripe`** — the bottom edge of a tool screen that is not a player.

- Height 4px, full width
- Gradient: `{colors.primary}` 0%, `{colors.sunshine-800}` 30%, `{colors.sunshine-500}` 55%, `{colors.yellow-saturated}` 78%, `{colors.cream}` 100%

## Writing in the interface

- The headline names the tool or the result: Transcribe, Transcript saved, Find best parts
- The next paragraph says what happened or what to do, including where a file was saved
- A button label is the action: Choose a video, Open transcription, Back to home
- Spoken language choices read Auto-detect, English, Portuguese, Spanish
- Settings that repair a bad transcript stay under “If the words come out wrong,” with names a person can understand
- When a step is finished, say what was saved and where. Offer Home as the way to pick another tool. Do not line up the other tools as buttons on that screen

## Do's and don'ts

### Do

- Use `{colors.primary}` for the primary button, the selected border, and the start of the sunset stripe
- Pair Noto Serif headlines with Inter for every other string
- Keep buttons at `{rounded.md}` and cards at `{rounded.lg}`
- Put Home at the top left of a tool screen
- Explain a tool before asking for a video
- Show a saved file as a path next to the video, and let the person open that file
- Keep the sunset stripe to 4px, on tool screens that are not built around the video player

### Don't

- Don't add a second accent color for buttons
- Don't use round pill buttons for actions
- Don't use a native dropdown for a short list of choices
- Don't stretch Home cards to fill the window
- Don't put a line under the title bar
- Don't describe other products' marketing sites in this file. This system is for the desktop app

## Window sizes

| Width | What changes |
|---|---|
| Under 760px | The home grid becomes one column. Tool sheets use the full width, with `{spacing.lg}` side padding. |
| 760–1100px | Home stays two columns. Display type uses the smaller end of its clamp, near 40px. |
| Over 1100px | Home headline can reach 56px. Tool titles can reach 52px. The sheet stays about 720px and remains centered in the column. |

Buttons on a tool screen are at least 48px tall. Fields and option buttons are at least 44px tall. The Home control is 36px tall and keeps a clear hit area.

The sunset stripe stays full width at every size.

## Iteration guide

1. Change one component at a time
2. Use the token names in this file (`{colors.primary}`, `{typography.display}`)
3. Run `npx @google/design.md lint DESIGN.md` after an edit to the front matter
4. Add a new variant as its own entry under `components:`
5. Body copy uses `{typography.body-md}`. The sentence under a tool title uses `{typography.subtitle}`
6. Keep `{colors.primary}` on the primary action, the selected state, and the sunset stripe
7. Cards use `{rounded.lg}`. Buttons and fields use `{rounded.md}`

## Known gaps

- Motion timings are not specified. A short fade near 150ms is enough for a panel opening
- The video player chrome (transport, crop frame, caption preview) is specified only by the same colors and type. Its layout follows the video
- There is no separate light theme. The window stays dark
