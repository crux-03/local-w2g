<script lang="ts">
    import type { WidgetEntry, WidgetStateUpload } from "$lib/api/types";
    import { formatBytes, formatDuration } from "$lib/helpers/format";
    import { userStore } from "$src/lib/stores/users.svelte";

    let { message, data }: { message: WidgetEntry; data: WidgetStateUpload } =
        $props();

    let done = $derived(message.kind.done);

    let percent = $derived(
        data.bytes_total > 0
            ? Math.min(100, (data.bytes_done / data.bytes_total) * 100)
            : 0,
    );

    let user = $derived(userStore.users.find((u) => u.id === data.uploader));

    let date = $derived(new Date(message.timestamp));

    let completedAt: number | undefined = $state();
    $effect(() => {
        if (message.kind.done && completedAt === undefined) {
            completedAt = Date.now();
        }
    });

    let durationMs = $derived(
        completedAt !== undefined ? completedAt - message.timestamp : undefined,
    );
</script>

<div class="widget" class:done>
    <div class="widget-header">
        <span class="widget-label">{done ? "Uploaded" : "Uploading"}</span>
        <span class="widget-filename" title={data.filename}>
            {data.filename}
        </span>
    </div>

    <!-- placeholder: wire up sender/uploader display name -->
    <div class="widget-submeta">
        by <span class="placeholder"
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

    {#if !done}
        <div class="widget-footer">
            <span
                >{formatBytes(data.bytes_done)} / {formatBytes(
                    data.bytes_total,
                )}</span
            >
            <span>{percent.toFixed(0)}%</span>
        </div>
    {:else}
        <div class="widget-footer">
            <span class="widget-duration">
                {date.toLocaleTimeString()} •
                {#if durationMs !== undefined}
                    <b>took {formatDuration(durationMs)}</b>
                {/if}
            </span>
            <!-- placeholder: swap message.id for the server-assigned file id -->
            <span class="widget-assigned-id">
                id <code>{message.id}</code>
            </span>
        </div>
    {/if}
</div>

<style>
    .widget {
        display: flex;
        flex-direction: column;
        min-width: 0;
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
        width: 100%;
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
    .placeholder {
        color: var(--twilight-300);
        font-weight: bold;
        font-style: italic;
    }

    .progress {
        height: 4px;
        background: var(--color-border);
        border-radius: var(--radius-full);
        overflow: hidden;
    }
    .progress-bar {
        height: 100%;
        background: var(--color-accent);
        transition:
            width var(--duration-fast) var(--ease-out),
            background-color var(--duration-normal) var(--ease-out);
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
    .widget-duration {
        color: var(--color-text-muted);
    }
    .widget-assigned-id {
        font-size: calc(var(--text-xs) * 0.9);
        color: var(--color-text-subtle);
    }
    .widget-assigned-id code {
        font-family: var(--font-mono);
    }
</style>
