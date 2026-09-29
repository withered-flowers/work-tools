<script lang="ts">
  import Icon from "./Icon.svelte";
  import { onMount, onDestroy } from "svelte";

  let {
    value = $bindable("2026-12-31T23:59"),
    compact = false,
    id = "",
    onchange = () => {},
  }: {
    value?: string;
    compact?: boolean;
    id?: string;
    onchange?: (val: string) => void;
  } = $props();

  let dateInputRef = $state<HTMLInputElement | null>(null);
  let clockBtnRef = $state<HTMLButtonElement | null>(null);
  let popoverRef = $state<HTMLDivElement | null>(null);

  let isClockOpen = $state(false);
  let clockMode = $state<"hour" | "minute">("hour");
  let popoverTop = $state(0);
  let popoverLeft = $state(0);

  // Parse current value
  let datePart = $state("2026-12-31");
  let timePart = $state("23:59");
  let hourNum = $state(23);
  let minuteNum = $state(59);

  // Keep internal state in sync with external value
  $effect(() => {
    const raw = (value || "").trim();
    if (/^(\d{4}-\d{2}-\d{2})/.test(raw)) {
      const matchDate = raw.match(/^(\d{4}-\d{2}-\d{2})/);
      if (matchDate) {
        datePart = matchDate[1];
      }
    }
    if (/[\sT](\d{2}:\d{2})/.test(raw)) {
      const matchTime = raw.match(/[\sT](\d{2}:\d{2})/);
      if (matchTime) {
        timePart = matchTime[1];
        const [h, m] = timePart.split(":").map(Number);
        hourNum = isNaN(h) ? 23 : Math.min(23, Math.max(0, h));
        minuteNum = isNaN(m) ? 59 : Math.min(59, Math.max(0, m));
      }
    }
  });

  function updateCombinedValue(newDate: string, newTime: string) {
    datePart = newDate;
    timePart = newTime;
    value = `${newDate}T${newTime}`;
    onchange(value);
  }

  function handleDateInput(e: Event) {
    const input = e.target as HTMLInputElement;
    if (input.value) {
      updateCombinedValue(input.value, timePart);
    }
  }

  function openNativeDatePicker() {
    if (dateInputRef) {
      try {
        if (typeof dateInputRef.showPicker === "function") {
          dateInputRef.showPicker();
        } else {
          dateInputRef.focus();
        }
      } catch {
        dateInputRef.focus();
      }
    }
  }

  function toggleClock(e?: MouseEvent) {
    if (e) {
      e.stopPropagation();
    }
    if (isClockOpen) {
      isClockOpen = false;
      return;
    }

    if (clockBtnRef) {
      const rect = clockBtnRef.getBoundingClientRect();
      const popoverWidth = 280;
      const popoverHeight = 350;

      let top = rect.bottom + 6;
      if (top + popoverHeight > window.innerHeight) {
        top = Math.max(10, rect.top - popoverHeight - 6);
      }

      let left = rect.left - 70;
      if (left + popoverWidth > window.innerWidth - 10) {
        left = window.innerWidth - popoverWidth - 10;
      }
      if (left < 10) left = 10;

      popoverTop = top;
      popoverLeft = left;
    }

    clockMode = "hour";
    isClockOpen = true;
  }

  function selectHour(h: number) {
    hourNum = h;
    const formattedTime = `${String(hourNum).padStart(2, "0")}:${String(minuteNum).padStart(2, "0")}`;
    updateCombinedValue(datePart, formattedTime);
    clockMode = "minute";
  }

  function selectMinute(m: number) {
    minuteNum = m;
    const formattedTime = `${String(hourNum).padStart(2, "0")}:${String(minuteNum).padStart(2, "0")}`;
    updateCombinedValue(datePart, formattedTime);
  }

  function applyPreset(presetTime: string) {
    const [h, m] = presetTime.split(":").map(Number);
    hourNum = h;
    minuteNum = m;
    updateCombinedValue(datePart, presetTime);
  }

  function stepHour(delta: number) {
    hourNum = (hourNum + delta + 24) % 24;
    const formattedTime = `${String(hourNum).padStart(2, "0")}:${String(minuteNum).padStart(2, "0")}`;
    updateCombinedValue(datePart, formattedTime);
  }

  function stepMinute(delta: number) {
    minuteNum = (minuteNum + delta + 60) % 60;
    const formattedTime = `${String(hourNum).padStart(2, "0")}:${String(minuteNum).padStart(2, "0")}`;
    updateCombinedValue(datePart, formattedTime);
  }

  function setNow() {
    const now = new Date();
    hourNum = now.getHours();
    minuteNum = now.getMinutes();
    const formattedTime = `${String(hourNum).padStart(2, "0")}:${String(minuteNum).padStart(2, "0")}`;
    const formattedDate = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
    updateCombinedValue(formattedDate, formattedTime);
  }

  function handleDirectTimeInput(e: Event) {
    const input = e.target as HTMLInputElement;
    if (input.value && /^\d{2}:\d{2}$/.test(input.value)) {
      const [h, m] = input.value.split(":").map(Number);
      hourNum = h;
      minuteNum = m;
      updateCombinedValue(datePart, input.value);
    }
  }

  function closeClock() {
    isClockOpen = false;
  }

  function handleWindowClick(e: MouseEvent) {
    if (!isClockOpen) return;
    const target = e.target as Node;
    if (
      popoverRef &&
      !popoverRef.contains(target) &&
      clockBtnRef &&
      !clockBtnRef.contains(target)
    ) {
      isClockOpen = false;
    }
  }

  function handleWindowKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape" && isClockOpen) {
      isClockOpen = false;
    }
  }

  onMount(() => {
    window.addEventListener("pointerdown", handleWindowClick);
    window.addEventListener("keydown", handleWindowKeyDown);
  });

  onDestroy(() => {
    window.removeEventListener("pointerdown", handleWindowClick);
    window.removeEventListener("keydown", handleWindowKeyDown);
  });

  // Clock Dial Geometry
  // 12 inner ring positions (1 to 12)
  const innerHours = [12, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
  // 12 outer ring positions (00, 13 to 23)
  const outerHours = [0, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23];
  // 12 minute positions (00, 05, 10, ..., 55)
  const minuteIntervals = [0, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 55];

  function getCoords(index: number, total: number, radius: number, cx = 95, cy = 95) {
    const angle = (index * (360 / total) - 90) * (Math.PI / 180);
    return {
      x: cx + radius * Math.cos(angle),
      y: cy + radius * Math.sin(angle),
    };
  }

  // Hand angle calculation
  const handAngle = $derived(() => {
    if (clockMode === "hour") {
      return (hourNum % 12) * 30;
    } else {
      return minuteNum * 6;
    }
  });

  const handLength = $derived(() => {
    if (clockMode === "hour") {
      return hourNum === 0 || hourNum >= 13 ? 72 : 46;
    }
    return 68;
  });
</script>

<div class="dt-picker-wrapper {compact ? 'dt-compact' : ''}">
  <!-- Composite Input Bar: Date on Left, Time on Right -->
  <div class="dt-input-group">
    <!-- Date Segment -->
    <div class="dt-segment dt-date-segment">
      <button
        type="button"
        class="dt-icon-btn dt-date-btn"
        onclick={openNativeDatePicker}
        title="Choose date via Calendar"
        aria-label="Choose date via Calendar"
      >
        <Icon name="calendar" size={compact ? 12 : 14} color="#6366f1" />
      </button>
      <input
        type="date"
        bind:this={dateInputRef}
        value={datePart}
        onchange={handleDateInput}
        {id}
        class="dt-native-input dt-date-input"
        title="Click to choose date from Calendar"
      />
    </div>

    <!-- Divider -->
    <div class="dt-divider" aria-hidden="true"></div>

    <!-- Time Segment -->
    <div class="dt-segment dt-time-segment">
      <button
        type="button"
        bind:this={clockBtnRef}
        class="dt-icon-btn dt-time-btn {isClockOpen ? 'btn-active' : ''}"
        onclick={toggleClock}
        title="Choose time via Clock"
        aria-label="Choose time via Clock"
      >
        <Icon name="clock" size={compact ? 12 : 14} color="#0284c7" />
        <span class="dt-time-text">{timePart}</span>
      </button>
    </div>
  </div>

  <!-- Interactive Clock Popover -->
  {#if isClockOpen}
    <div
      bind:this={popoverRef}
      class="dt-clock-popover"
      style="top: {popoverTop}px; left: {popoverLeft}px;"
    >
      <!-- Popover Header -->
      <div class="popover-header">
        <div class="popover-title">
          <Icon name="clock" size={13} color="#0284c7" />
          <span>Deadline Time</span>
        </div>
        <button
          type="button"
          class="btn-popover-close"
          onclick={closeClock}
          title="Close Clock"
          aria-label="Close"
        >
          <Icon name="cross" size={12} color="#64748b" />
        </button>
      </div>

      <!-- Digital Display & Mode Toggle -->
      <div class="digital-time-display">
        <div class="time-stepper">
          <button
            type="button"
            class="stepper-btn"
            onclick={() => (clockMode === "hour" ? stepHour(1) : stepMinute(1))}
            title="Increment"
          >
            +
          </button>
        </div>

        <div class="time-readout">
          <button
            type="button"
            class="time-pill {clockMode === 'hour' ? 'time-pill-active' : ''}"
            onclick={() => (clockMode = "hour")}
            title="Select Hour"
          >
            {String(hourNum).padStart(2, "0")}
          </button>
          <span class="time-colon">:</span>
          <button
            type="button"
            class="time-pill {clockMode === 'minute' ? 'time-pill-active' : ''}"
            onclick={() => (clockMode = "minute")}
            title="Select Minute"
          >
            {String(minuteNum).padStart(2, "0")}
          </button>
        </div>

        <div class="time-stepper">
          <button
            type="button"
            class="stepper-btn"
            onclick={() => (clockMode === "hour" ? stepHour(-1) : stepMinute(-1))}
            title="Decrement"
          >
            -
          </button>
        </div>
      </div>

      <!-- Mode Tab Bar -->
      <div class="mode-tabs">
        <button
          type="button"
          class="mode-tab {clockMode === 'hour' ? 'active-tab' : ''}"
          onclick={() => (clockMode = "hour")}
        >
          Hours (24h)
        </button>
        <button
          type="button"
          class="mode-tab {clockMode === 'minute' ? 'active-tab' : ''}"
          onclick={() => (clockMode = "minute")}
        >
          Minutes
        </button>
      </div>

      <!-- Visual Clock Dial -->
      <div class="clock-dial-container">
        <svg
          class="clock-svg"
          viewBox="0 0 190 190"
          width="190"
          height="190"
        >
          <!-- Clock Face Background -->
          <circle cx="95" cy="95" r="92" class="clock-face-circle" />

          <!-- Center Pivot -->
          <circle cx="95" cy="95" r="4" class="clock-pivot" />

          <!-- Clock Hand Line -->
          <line
            x1="95"
            y1="95"
            x2={95 + handLength() * Math.sin(handAngle() * (Math.PI / 180))}
            y2={95 - handLength() * Math.cos(handAngle() * (Math.PI / 180))}
            class="clock-hand"
          />

          <!-- Hand Target Indicator -->
          <circle
            cx={95 + handLength() * Math.sin(handAngle() * (Math.PI / 180))}
            cy={95 - handLength() * Math.cos(handAngle() * (Math.PI / 180))}
            r="12"
            class="clock-hand-endpoint"
          />

          {#if clockMode === "hour"}
            <!-- Outer Ring Hours (00, 13-23) -->
            {#each outerHours as h, i}
              {@const pos = getCoords(i, 12, 72)}
              <g
                class="clock-num-group {hourNum === h ? 'clock-num-selected' : ''}"
                onclick={() => selectHour(h)}
                role="button"
                tabindex="0"
                onkeydown={(e) => e.key === "Enter" && selectHour(h)}
              >
                <circle cx={pos.x} cy={pos.y} r="10" class="clock-hit-circle" />
                <text x={pos.x} y={pos.y + 3.5} text-anchor="middle" class="clock-num-text outer-hour">
                  {String(h).padStart(2, "0")}
                </text>
              </g>
            {/each}

            <!-- Inner Ring Hours (1-12) -->
            {#each innerHours as h, i}
              {@const pos = getCoords(i, 12, 46)}
              <g
                class="clock-num-group {hourNum === h ? 'clock-num-selected' : ''}"
                onclick={() => selectHour(h)}
                role="button"
                tabindex="0"
                onkeydown={(e) => e.key === "Enter" && selectHour(h)}
              >
                <circle cx={pos.x} cy={pos.y} r="9" class="clock-hit-circle" />
                <text x={pos.x} y={pos.y + 3} text-anchor="middle" class="clock-num-text inner-hour">
                  {h}
                </text>
              </g>
            {/each}
          {:else}
            <!-- Minute Ring (00, 05, ..., 55) -->
            {#each minuteIntervals as m, i}
              {@const pos = getCoords(i, 12, 68)}
              <g
                class="clock-num-group {minuteNum === m ? 'clock-num-selected' : ''}"
                onclick={() => selectMinute(m)}
                role="button"
                tabindex="0"
                onkeydown={(e) => e.key === "Enter" && selectMinute(m)}
              >
                <circle cx={pos.x} cy={pos.y} r="11" class="clock-hit-circle" />
                <text x={pos.x} y={pos.y + 3.5} text-anchor="middle" class="clock-num-text minute-text">
                  {String(m).padStart(2, "0")}
                </text>
              </g>
            {/each}
          {/if}
        </svg>

        {#if clockMode === "minute"}
          <!-- Quick 59 End of Hour Badge -->
          <button
            type="button"
            class="btn-badge-59 {minuteNum === 59 ? 'active-59' : ''}"
            onclick={() => selectMinute(59)}
            title="Set exact 59 min (Standard Deadline)"
          >
            :59 (End)
          </button>
        {/if}
      </div>

      <!-- Quick Assignment Deadline Presets -->
      <div class="popover-presets">
        <span class="preset-label">Presets:</span>
        <div class="preset-pills">
          <button
            type="button"
            class="preset-btn {timePart === '23:59' ? 'preset-active' : ''}"
            onclick={() => applyPreset("23:59")}
            title="Standard Assignment Deadline (23:59)"
          >
            23:59
          </button>
          <button
            type="button"
            class="preset-btn {timePart === '18:00' ? 'preset-active' : ''}"
            onclick={() => applyPreset("18:00")}
            title="Evening (18:00)"
          >
            18:00
          </button>
          <button
            type="button"
            class="preset-btn {timePart === '17:00' ? 'preset-active' : ''}"
            onclick={() => applyPreset("17:00")}
            title="End of Work (17:00)"
          >
            17:00
          </button>
          <button
            type="button"
            class="preset-btn {timePart === '12:00' ? 'preset-active' : ''}"
            onclick={() => applyPreset("12:00")}
            title="Noon (12:00)"
          >
            12:00
          </button>
        </div>
      </div>

      <!-- Bottom Manual Input & Actions -->
      <div class="popover-footer">
        <div class="manual-input-wrap">
          <span class="manual-label">Type:</span>
          <input
            type="time"
            value={timePart}
            onchange={handleDirectTimeInput}
            class="manual-time-input"
            title="Direct time input"
          />
        </div>

        <div class="footer-actions">
          <button type="button" class="btn-footer-now" onclick={setNow} title="Set to current local time">
            Now
          </button>
          <button type="button" class="btn-footer-apply" onclick={closeClock}>
            Done
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .dt-picker-wrapper {
    position: relative;
    display: inline-block;
    width: 100%;
    vertical-align: middle;
  }

  .dt-input-group {
    display: flex;
    align-items: center;
    background: #f8fafc;
    border: 1px solid #cbd5e1;
    border-radius: 8px;
    height: 42px;
    box-sizing: border-box;
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
    width: 100%;
  }

  .dt-compact .dt-input-group {
    height: 34px;
    border-radius: 6px;
    background: #ffffff;
    border-color: #e2e8f0;
  }

  .dt-input-group:focus-within,
  .dt-input-group:hover {
    border-color: #6366f1;
    background: #ffffff;
    box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.12);
  }

  .dt-segment {
    display: flex;
    align-items: center;
    height: 100%;
  }

  .dt-date-segment {
    flex: 1 1 auto;
    position: relative;
    padding-left: 0.5rem;
    min-width: 125px;
  }

  .dt-time-segment {
    flex: 0 0 auto;
    padding-right: 0.4rem;
  }

  .dt-icon-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0.25rem;
    border-radius: 4px;
    transition: background-color 0.15s ease;
    color: #475569;
  }

  .dt-icon-btn:hover {
    background: #f1f5f9;
  }

  .dt-native-input {
    border: none;
    background: transparent;
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.8rem;
    color: #1e293b;
    outline: none;
    cursor: pointer;
    width: 100%;
    height: 100%;
    padding: 0 0.25rem;
  }

  .dt-compact .dt-native-input {
    font-size: 0.75rem;
  }

  .dt-divider {
    width: 1px;
    height: 20px;
    background: #e2e8f0;
    margin: 0 0.35rem;
    flex-shrink: 0;
  }

  .dt-time-btn {
    gap: 0.35rem;
    background: #f0f9ff;
    border: 1px solid #bae6fd;
    border-radius: 6px;
    padding: 0.2rem 0.5rem;
    height: 28px;
    color: #0369a1;
    font-weight: 600;
  }

  .dt-compact .dt-time-btn {
    height: 24px;
    padding: 0.1rem 0.4rem;
    font-size: 0.725rem;
  }

  .dt-time-btn:hover,
  .dt-time-btn.btn-active {
    background: #e0f2fe;
    border-color: #38bdf8;
    color: #0284c7;
    box-shadow: 0 1px 3px rgba(2, 132, 199, 0.15);
  }

  .dt-time-text {
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.775rem;
  }

  .dt-compact .dt-time-text {
    font-size: 0.725rem;
  }

  /* Clock Popover */
  .dt-clock-popover {
    position: fixed;
    z-index: 9999;
    width: 280px;
    background: #ffffff;
    border: 1px solid #e2e8f0;
    border-radius: 12px;
    box-shadow: 0 12px 30px -4px rgba(15, 23, 42, 0.18), 0 4px 10px -2px rgba(15, 23, 42, 0.08);
    padding: 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
    animation: popoverFadeIn 0.16s ease-out;
  }

  @keyframes popoverFadeIn {
    from {
      opacity: 0;
      transform: translateY(-6px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .popover-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-bottom: 0.4rem;
    border-bottom: 1px solid #f1f5f9;
  }

  .popover-title {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.775rem;
    font-weight: 700;
    color: #1e293b;
  }

  .btn-popover-close {
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0.2rem;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .btn-popover-close:hover {
    background: #f1f5f9;
  }

  /* Digital Readout */
  .digital-time-display {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.2rem 0;
  }

  .time-readout {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .time-pill {
    font-family: 'JetBrains Mono', monospace;
    font-size: 1.35rem;
    font-weight: 700;
    color: #334155;
    background: #f1f5f9;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    padding: 0.2rem 0.55rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .time-pill:hover {
    background: #e2e8f0;
  }

  .time-pill-active {
    background: #eef2ff !important;
    border-color: #6366f1 !important;
    color: #4f46e5 !important;
    box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.2);
  }

  .time-colon {
    font-family: 'JetBrains Mono', monospace;
    font-size: 1.3rem;
    font-weight: 700;
    color: #64748b;
  }

  .stepper-btn {
    background: #f8fafc;
    border: 1px solid #cbd5e1;
    border-radius: 6px;
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 700;
    font-size: 0.9rem;
    color: #475569;
    cursor: pointer;
  }

  .stepper-btn:hover {
    background: #e2e8f0;
    color: #0f172a;
  }

  /* Mode Tabs */
  .mode-tabs {
    display: grid;
    grid-template-columns: 1fr 1fr;
    background: #f1f5f9;
    border-radius: 6px;
    padding: 2px;
    gap: 2px;
  }

  .mode-tab {
    background: transparent;
    border: none;
    padding: 0.3rem;
    font-size: 0.725rem;
    font-weight: 600;
    color: #64748b;
    border-radius: 5px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .mode-tab.active-tab {
    background: #ffffff;
    color: #4f46e5;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
  }

  /* Clock Dial */
  .clock-dial-container {
    position: relative;
    display: flex;
    justify-content: center;
    align-items: center;
    padding: 0.35rem 0;
  }

  .clock-svg {
    background: #fafaf9;
    border-radius: 50%;
    cursor: pointer;
    user-select: none;
  }

  .clock-face-circle {
    fill: #ffffff;
    stroke: #e2e8f0;
    stroke-width: 1.5;
  }

  .clock-pivot {
    fill: #6366f1;
  }

  .clock-hand {
    stroke: #6366f1;
    stroke-width: 2;
    stroke-linecap: round;
  }

  .clock-hand-endpoint {
    fill: #4f46e5;
    stroke: #ffffff;
    stroke-width: 2;
    opacity: 0.85;
  }

  .clock-num-group {
    cursor: pointer;
  }

  .clock-hit-circle {
    fill: transparent;
    transition: fill 0.12s ease;
  }

  .clock-num-group:hover .clock-hit-circle {
    fill: rgba(99, 102, 241, 0.15);
  }

  .clock-num-text {
    font-family: 'JetBrains Mono', monospace;
    font-weight: 600;
    fill: #334155;
    pointer-events: none;
  }

  .outer-hour {
    font-size: 9.5px;
    fill: #64748b;
  }

  .inner-hour {
    font-size: 10px;
    font-weight: 700;
    fill: #1e293b;
  }

  .minute-text {
    font-size: 9.5px;
  }

  .clock-num-selected .clock-num-text {
    fill: #ffffff !important;
    font-weight: 800;
  }

  .btn-badge-59 {
    position: absolute;
    bottom: 2px;
    right: 8px;
    background: #fdf2f8;
    border: 1px solid #fbcfe8;
    color: #db2777;
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.7rem;
    font-weight: 700;
    border-radius: 9999px;
    padding: 0.15rem 0.5rem;
    cursor: pointer;
    box-shadow: 0 1px 2px rgba(219, 39, 119, 0.1);
  }

  .btn-badge-59:hover,
  .btn-badge-59.active-59 {
    background: #db2777;
    color: #ffffff;
    border-color: #be185d;
  }

  /* Presets Bar */
  .popover-presets {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.7rem;
  }

  .preset-label {
    color: #64748b;
    font-weight: 600;
  }

  .preset-pills {
    display: flex;
    gap: 0.25rem;
    flex-wrap: wrap;
  }

  .preset-btn {
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.675rem;
    font-weight: 600;
    background: #f8fafc;
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    padding: 0.15rem 0.4rem;
    color: #334155;
    cursor: pointer;
    transition: all 0.12s ease;
  }

  .preset-btn:hover {
    background: #eef2ff;
    border-color: #a5b4fc;
    color: #4f46e5;
  }

  .preset-btn.preset-active {
    background: #4f46e5;
    border-color: #4338ca;
    color: #ffffff;
  }

  /* Popover Footer */
  .popover-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 0.4rem;
    border-top: 1px solid #f1f5f9;
  }

  .manual-input-wrap {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }

  .manual-label {
    font-size: 0.7rem;
    color: #64748b;
  }

  .manual-time-input {
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.75rem;
    padding: 0.15rem 0.35rem;
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    color: #1e293b;
    background: #f8fafc;
    outline: none;
    width: 75px;
  }

  .footer-actions {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }

  .btn-footer-now {
    background: #f1f5f9;
    border: 1px solid #cbd5e1;
    border-radius: 5px;
    padding: 0.25rem 0.55rem;
    font-size: 0.725rem;
    font-weight: 600;
    color: #475569;
    cursor: pointer;
  }

  .btn-footer-now:hover {
    background: #e2e8f0;
    color: #0f172a;
  }

  .btn-footer-apply {
    background: #4f46e5;
    border: 1px solid #4338ca;
    border-radius: 5px;
    padding: 0.25rem 0.75rem;
    font-size: 0.725rem;
    font-weight: 600;
    color: #ffffff;
    cursor: pointer;
    box-shadow: 0 1px 2px rgba(79, 70, 229, 0.2);
  }

  .btn-footer-apply:hover {
    background: #4338ca;
  }
</style>
