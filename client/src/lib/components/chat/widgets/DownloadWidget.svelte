<script lang="ts">
    import type { WidgetEntry, WidgetStateDownload } from "$lib/api/types";
    import { formatBytes } from "$lib/helpers/format";
    import { userStore } from "$src/lib/stores/users.svelte";

    let { message, data }: { message: WidgetEntry; data: WidgetStateDownload } =
        $props();

    let done = $derived(message.kind.done);

    let user = $derived(userStore.users.find((u) => u.id === data.reporter));

    let percent = $derived(
        data.bytes_total > 0
            ? Math.min(100, (data.bytes_done / data.bytes_total) * 100)
            : 0,
    );
</script>

<div class="widget" class:done>
    <div class="widget-header">
        <span class="widget-label">{done ? "Downloaded" : "Downloading"}</span>
        <span class="widget-filename" title={data.filename}>
            {data.filename}
        </span>
    </div>

    <div class="widget-submeta">
        from <span class="placeholder"
            >{user?.display_name ?? user?.id ?? "System"}</span
        >
    </div>

    <div
        class="progress"
        role="progressbar"
        aria-valuenow={percent}
        aria-valuemin={0}
        aria-valuemax={100}
    >
        <div class="progress-bar" style:width="{percent}%"></div>
    </div>

    <div class="widget-footer">
        <span
            >{formatBytes(data.bytes_done)} / {formatBytes(
                data.bytes_total,
            )}</span
        >
        <span>{percent.toFixed(0)}%</span>
    </div>
</div>

<style>
    .widget {
        display: flex;
        flex-direction: column;
        gap: var(--space-2);
        padding: var(--space-3) var(--space-4);
        border: var(--border-thin) solid var(--color-border);
        border-radius: var(--radius-md);
        background: var(--color-surface);
        color: var(--color-text);
        font-family: var(--font-sans);
        font-size: var(--text-sm);
        box-shadow: var(--shadow-xs);
    }
    .widget.done {
        background: var(--color-bg-subtle);
    }

    .widget-header {
        display: flex;
        align-items: baseline;
        justify-content: space-between;
        gap: var(--space-3);
    }
    .widget-label {
        flex-shrink: 0;
        font-size: var(--text-xs);
        font-weight: var(--font-medium);
        text-transform: uppercase;
        letter-spacing: var(--tracking-wide);
        color: var(--color-text-muted);
    }
    .widget.done .widget-label {
        color: var(--color-success);
    }
    .widget-filename {
        min-width: 0;
        font-weight: var(--font-medium);
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .widget-submeta {
        font-size: var(--text-xs);
        color: var(--color-text-muted);
    }

    .progress {
        height: 4px;
        background: var(--color-border);
        border-radius: var(--radius-full);
        overflow: hidden;
    }
    .progress-bar {
        height: 100%;
        background: var(--color-secondary);
        transition:
            width var(--duration-fast) var(--ease-out),
            background-color var(--duration-normal) var(--ease-out);
    }
    .placeholder {
        color: var(--twilight-300);
        font-weight: bold;
        font-style: italic;
    }
    .widget.done .progress-bar {
        background: var(--color-success);
    }

    .widget-footer {
        display: flex;
        align-items: baseline;
        justify-content: space-between;
        gap: var(--space-3);
        font-size: var(--text-xs);
        color: var(--color-text-muted);
        font-variant-numeric: tabular-nums;
    }
</style>
