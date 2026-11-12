# Search Logs Virtualized Results Table Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Scrolling the Search Logs results stays smooth with thousands of results; frame cost no longer grows with the result count.

**Architecture:** The results table becomes a gpui `list()` driven by a `ListState`. The list measures and builds only the rows near the viewport, each at its real height, since ERROR rows can be one or two lines. Render no longer deep-clones the response every frame: `SearchStatus::Success` holds an `Rc`, and a flat row index maps to its entry by skipping over whole containers.

**Tech Stack:** Rust 2024, gpui-kit =0.9.1 (gpui-pre 0.3.12: `list`, `ListState`, `ListAlignment`, `Context::processor`).

**Spec:** `docs/superpowers/specs/2026-11-11-gpui-full-rust-migration-design.md` (parity rules). The root cause analysis is below.

## Root cause (measured)

- **Every row is rebuilt every frame.** `src/app/log_search.rs:553-690` builds one element tree per result row inside a plain `overflow_y_scroll` div. gpui rebuilds, lays out and prepaints every row on every frame.
  - That includes each scroll tick, each hover change, and each `Store` notify, since `observe_in` re-renders on every polling tick.
- **The whole response is copied every frame.** `render()` also runs `self.state.visible(..).clone()` at `log_search.rs:239`, which copies every entry's `message` and `raw` strings.
- **Measured** (release build, test platform, INFO rows, 10 scroll frames):
  - 100 rows: 24 ms per frame;
  - 1,000 rows: 219 ms per frame;
  - 5,000 rows: 1,019 ms per frame.
- **Why it freezes:** the cost is linear, so a few thousand matches freeze the app. The backend does not cap matches (`src/k8s/logs.rs` `search_deployment_logs_with_client`).
- **Why React was fine:** the browser keeps the DOM and only composites on scroll.
- **Precedent:** Log Viewer already virtualizes with `v_virtual_list`, but its row heights are fixed. Search rows are not, so this fix uses `list()`.

## Global Constraints

- **Dependencies:** `gpui-kit = "=0.9.1"` stays exactly pinned, and no new dependencies are added.
- **Parity with React `LogSearch.tsx`:**
  - same columns, widths, colours, hover background and row borders;
  - the header stays fixed while the rows scroll;
  - clicking a row opens `LogDetailModal` with that row's entry;
  - ERROR rows keep the padded mark and the 2-line clamp (`mark_text`).
- **Element ids:** keep `search-table-scroll`, `("search-row", i)`, `("search-pod-cell", i)`, `("search-pod", i)` and `("search-container", i)` (tests use them).
- **No commits and no push.** The owner commits later. `log_search.rs` and `search_state.rs` are untracked Task 13 files, so this change folds into the Task 13 commit.
- **Done means:** `cargo test` passes in full and `cargo build` reports 0 warnings.

## Review Focus

1. **A new search returns a different number of rows while the user is scrolled down.** The list shows the new rows from the top, with no stale heights and no out-of-range rows. Pinned by `new_search_resets_rows_to_top` (Task 2).
2. **The user drags the Pod column wider.** ERROR rows that now wrap grow to two lines, and rows never overlap. Pinned by `pod_resize_remeasures_error_rows` (Task 2).
3. **Results span several pods or containers.** Row *i* maps to the same entry as React's nested `results → entries` order, so clicking opens the right log. Pinned by `entry_at_skips_whole_containers` (Task 1).
4. **The result count grows to 5,000 or more.** Frame time stays flat rather than linear. Pinned by the ignored probe `search_scroll_frame_time_probe` (Task 2, run manually).
5. **The user flicks a trackpad with momentum.** Scrolling stays smooth and reaches the last row.
   - gpui clamps each scroll event to rows that are already measured, so the 400 px overdraw sets the maximum speed per frame.
   - Owner manual check (Task 2, Step 6).

---

### Task 1: Cheap per-frame data (`Rc` response and `entry_at`)

**Files:**
- Modify: `src/app/search_state.rs`. `SearchStatus::Success(Snapshot, Rc<LogSearchResponse>)`; `finish` wraps it in `Rc::new`; existing tests construct it with `Rc::new(..)`.
- Modify: `src/app/log_search.rs`. Add `entry_at` and its test; callers of `Success(_, d)` keep working through `Deref`.

**Interfaces:**
- Produces: `SearchStatus::Success(Snapshot, std::rc::Rc<LogSearchResponse>)`. `SearchStatus` stays `Clone`, and cloning is now O(1).
- Produces: `pub fn entry_at(d: &LogSearchResponse, ix: usize) -> Option<(&LogSearchResult, &LogEntry)>` in `log_search.rs`.
  - The order is results order, then entries order.
  - It costs O(containers) and does not walk every entry.
- Produces: `pub fn row_count(d: &LogSearchResponse) -> usize`, the sum of `entries.len()`.

- [ ] **Step 1: Write the failing test** in `log_search.rs` `mod tests`

```rust
#[test]
fn entry_at_skips_whole_containers() {
    use crate::models::{LogEntry, LogSearchResponse, LogSearchResult};
    let e = |m: &str| LogEntry { timestamp: None, level: None, message: m.into(), raw: String::new(), is_json: false, pod_name: String::new(), container_name: String::new() };
    let r = |pod: &str, entries: Vec<LogEntry>| LogSearchResult { pod_name: pod.into(), container_name: "c".into(), total_matches: entries.len() as i32, entries };
    let d = LogSearchResponse { results: vec![r("p1", vec![e("a"), e("b")]), r("p2", vec![]), r("p3", vec![e("c")])], ..Default::default() };
    let got: Vec<_> = (0..4).map(|i| entry_at(&d, i).map(|(r, e)| (r.pod_name.as_str(), e.message.as_str()))).collect();
    assert_eq!(got, [Some(("p1", "a")), Some(("p1", "b")), Some(("p3", "c")), None]);
    assert_eq!(row_count(&d), 3);
}
```

`LogEntry` and `LogSearchResult` have no `Default`, so the test uses full struct literals on purpose. Do not add derives just for the test.

- [ ] **Step 2: Run the test to verify it fails.**
  Run `cargo test entry_at_skips_whole_containers`. Expected: a compile error, because `entry_at` and `row_count` are not defined.

- [ ] **Step 3: Implement the change.**
  - Add `entry_at` and `row_count`. `entry_at` loops over `d.results`, subtracting each `entries.len()` until `ix` falls inside one.
  - Switch `SearchStatus::Success` to `Rc<LogSearchResponse>`, and leave `render()`'s `.clone()` as it is, since it is now cheap.

- [ ] **Step 4: Run the tests to verify they pass.**
  Run `cargo test`. Expected: all pass, 0 warnings.

- [ ] **Step 5: No commit.** This follows the owner's standing instruction.

---

### Task 2: Virtualize the results table with `list()`

**Files:**
- Modify: `src/app/log_search.rs`, in these places:
  - the struct fields and `new()`;
  - the search `finish` callback (around line 199);
  - the pod-column drag handler (around line 766);
  - `render()`, at the rows loop on lines 553-690 and the `mark_h` calculation;
  - the `boot_search` test helper.

**Interfaces:**
- Consumes: `entry_at` and `row_count` (from Task 1).
- Produces:
  - a field `list: ListState`, initialised in `new()` with `ListState::new(0, ListAlignment::Top, px(400.))`;
  - `fn render_row(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) -> AnyElement`;
  - a test helper `fn boot_search_entries(cx: &mut gpui_kit::TestAppContext, entries: Vec<(&str, String)>) -> Env`, where each pair is (level, message). `boot_search(cx, n)` delegates to it with `("INFO", format!("line {i}"))`.

- [ ] **Step 1: Write the failing tests** in `log_search.rs` `mod tests`

```rust
#[gpui_kit::test]
fn search_table_builds_only_visible_rows(cx: &mut gpui_kit::TestAppContext) {
    use gpui_kit::test::TestWindowExt;
    let env = boot_search(cx, 5000);
    cx.update_window(env.handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.try_find(("search-row", 0usize)).is_some());
        assert!(w.try_find(("search-row", 200usize)).is_none(), "off-screen rows must not be built");
    }).unwrap();
}

#[gpui_kit::test]
fn search_table_scrolls_through_virtualized_rows(cx: &mut gpui_kit::TestAppContext) {
    use gpui_kit::{ScrollDelta, point, px, test::TestWindowExt};
    let env = boot_search(cx, 5000);
    cx.update_window(env.handle, |_, w, cx| {
        for _ in 0..30 { w.scroll("search-table-scroll", ScrollDelta::Pixels(point(px(0.), px(-300.))), cx); }
        assert!(w.try_find(("search-row", 0usize)).is_none());
        assert!((150..400usize).any(|i| w.try_find(("search-row", i)).is_some()));
    }).unwrap();
}

#[gpui_kit::test]
fn new_search_resets_rows_to_top(cx: &mut gpui_kit::TestAppContext) {
    // boot_search(cx, 5000); scroll 10 × -300 px; then
    // env.api.reply(Endpoint::SearchLogs, <3 INFO entries>); click "search-submit"; cx.run_until_parked();
    // render_frame → assert row 0 is Some, row 2 is Some, row 3 is None.
}

#[gpui_kit::test]
fn pod_resize_remeasures_error_rows(cx: &mut gpui_kit::TestAppContext) {
    // boot_search_entries(cx, vec![("ERROR", "e".repeat(40)), ("ERROR", "e".into())]);
    // before: h0 = row 0 height, h1 = row 1 height; assert_eq!(h0, h1) (precondition: 40 chars fit on one line at pod width 208).
    // drag "search-resize" by +292 px (pod 208 → 500, as in header_stays_visible_after_scrolling_results);
    // after: assert!(row0.height > h1) (it now wraps to 2 lines);
    //        assert!(row1.origin.y >= row0.origin.y + row0.height) (no overlap).
}

#[gpui_kit::test]
#[ignore] // perf probe: cargo test --release search_scroll_frame_time_probe -- --ignored --nocapture
fn search_scroll_frame_time_probe(cx: &mut gpui_kit::TestAppContext) {
    // for n in [100, 1000, 5000]: boot_search(cx, n); time 10 × (scroll -20 px + render_frame); eprintln!("rows={n}: {:?}/frame", avg).
}
```

The bodies given as comments are fixed assertions; write them in the same style as `header_stays_visible_after_scrolling_results`. The test text system is `NoopTextSystem`, where every ASCII glyph is 0.6 em (8.4 px at 14 px), so the wrap test is deterministic.

- [ ] **Step 2: Run the tests to verify they fail.**
  Run `cargo test -- search_table_ pod_resize new_search_resets`.
  Expected:
  - `search_table_builds_only_visible_rows` fails, because row 200 exists;
  - the scroll test fails, because row 0 is still built.
  - The other two may pass on the old code. They guard the new code, so keep them.

- [ ] **Step 3: Implement the changes.**
  - **Rows:** move the body of the per-row loop verbatim into `render_row`.
    - It looks up `entry_at` on the visible `Success` data and uses that snapshot's keyword.
    - It returns `div().into_any_element()` when the status is not `Success` or `ix` is out of range.
    - It computes `mark_h` there, which is cheap (font metrics).
  - **Table:** replace the rows div with
    `div().id("search-table-scroll").test_support().flex_1().min_h_0().child(list(self.list.clone(), cx.processor(|this, ix: usize, window, cx| this.render_row(ix, window, cx))).size_full())`.
    The wrapper no longer has `overflow_y_scroll`, because the list handles the scroll wheel.
  - **New results:** in the `finish` callback, after `state.finish`, call `this.list.reset(n)`. Here `n` is `row_count` of the visible `Success` data, or 0. Calling reset also scrolls the list back to the top.
  - **Pod column drag:** in the drag handler, after `pod_width` changes, call `this.list.remeasure()`.
  - **Window resize** needs no code: the list remeasures by itself when its width changes.

- [ ] **Step 4: Run the tests to verify they pass.**
  Run `cargo test`. Expected:
  - all tests pass, including `header_stays_visible_after_scrolling_results` and `error_row_with_match_renders_painted_mark`;
  - 0 warnings.

- [ ] **Step 5: Measure.**
  Run `cargo test --release search_scroll_frame_time_probe -- --ignored --nocapture`.
  Expected: the frame time for 5,000 rows is within 2× of the time for 100 rows. Before the fix the times were 24, 219 and 1,019 ms.

- [ ] **Step 6: Owner manual check.** In `cargo run`, open Search Logs and run a search that returns thousands of matches. Then check:
  - a fast trackpad flick is smooth and reaches the last row;
  - hover and row click work, and the modal shows the clicked row;
  - ERROR rows wrap or clamp as before;
  - a Pod column drag reflows the rows;
  - the app does not freeze on polling ticks.

- [ ] **Step 7: No commit.** This follows the owner's standing instruction. Add one line to the commit-split doc: "Task 13 follow-up: virtualized search table (folded into Task 13)".
