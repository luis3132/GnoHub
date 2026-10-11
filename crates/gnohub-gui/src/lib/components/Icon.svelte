<script lang="ts">
  // Iconos de trazo a 24×24.
  const PATHS: Record<string, string> = {
    mouse: "M12 3a6 6 0 0 1 6 6v6a6 6 0 0 1-12 0V9a6 6 0 0 1 6-6zM12 3v7M6 10h12",
    keyboard: "M3 6h18v12H3zM7 10h.01M11 10h.01M15 10h.01M7 14h10",
    gamepad: "M6 8h12a4 4 0 0 1 4 4v1a4 4 0 0 1-7 2.6L14 14h-4l-1 1.6A4 4 0 0 1 2 13v-1a4 4 0 0 1 4-4zM7 11v3M5.5 12.5h3M16 12h.01M18 13.5h.01",
    device: "M4 5h16v14H4zM8 9h8",
    macro: "M4 6h10M4 12h16M4 18h7M17 4l3 2-3 2M14 16l3 2 3-2",
    settings: "M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM12 2v3M12 19v3M4.2 4.2l2.1 2.1M17.7 17.7l2.1 2.1M2 12h3M19 12h3M4.2 19.8l2.1-2.1M17.7 6.3l2.1-2.1",
    plus: "M12 5v14M5 12h14",
    trash: "M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3",
    up: "M12 19V5M6 11l6-6 6 6",
    down: "M12 5v14M6 13l6 6 6-6",
    record: "M12 7a5 5 0 1 0 0 10 5 5 0 0 0 0-10z",
    stop: "M7 7h10v10H7z",
    target: "M12 3v3M12 18v3M3 12h3M18 12h3M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8z",
    refresh: "M20 11a8 8 0 0 0-14.6-4.5M4 4v3h3M4 13a8 8 0 0 0 14.6 4.5M20 20v-3h-3",
    x: "M6 6l12 12M18 6L6 18",
    check: "M5 12l5 5 9-10",
    alert: "M12 4l9 16H3zM12 10v4M12 17h.01",
    keycap: "M5 5h14v12H5zM5 17l2 2h10l2-2M9 10h6",
  };

  let { name, size = 18 }: { name: keyof typeof PATHS | string; size?: number } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.8"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d={PATHS[name] ?? ""} />
</svg>
