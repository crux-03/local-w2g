<script lang="ts">
    import { Check, CircleDashed, Download } from "@lucide/svelte";
    import { playlistStore } from "$lib/stores/playlist.svelte";
    import { messageStore } from "$lib/stores/messages.svelte";
    import type {
        Snowflake,
        UserReadinessView,
        WidgetStateDownload,
    } from "$lib/api/types";
    import type { VideoEntry } from "$lib/api/video";

    let {
        userId,
        readiness,
    }: {
        userId: Snowflake;
        readiness: UserReadinessView | undefined;
    } = $props();

    const entries = $derived(
        [...playlistStore.entries].sort((a, b) => a.order - b.order),
    );

    const userDownloads = $derived(
        messageStore.activeDownloads.filter((d) => d.reporter === userId),
    );

    function statusFor(videoId: Snowflake) {
        return readiness?.videos[videoId]?.status ?? "not_started";
    }

    // TODO: replace with `d.video_id === video.id` once the server adds it.
    function findDownload(video: VideoEntry): WidgetStateDownload | undefined {
        const name = video.display_name.toLowerCase();
        return userDownloads.find((d) =>
            d.filename.toLowerCase().includes(name),
        );
    }

    function percentOf(d: WidgetStateDownload): number {
        const total = Math.max(1, d.bytes_total);
        return Math.min(100, Math.floor((d.bytes_done / total) * 100));
    }
</script>

{#if entries.length === 0}
    <p class="empty">No videos in playlist.</p>
{:else}
    <ul class="video-list">
        {#each entries as video (video.id)}
            {@const status = statusFor(video.id)}
            {@const download =
                status === "pending" ? findDownload(video) : undefined}
            <li
                class="video-row"
                class:on-device={status === "on_device"}
                class:pending={status === "pending"}
                class:not-started={status === "not_started"}
            >
                <div class="video-row-main">
                    <span class="status-icon" aria-hidden="true">
                        {#if status === "on_device"}
                            <Check />
                        {:else if status === "pending"}
                            <Download />
                        {:else}
                            <CircleDashed />
                        {/if}
                    </span>
                    <span class="title">{video.display_name}</span>
                    {#if download}
                        <span class="percent">{percentOf(download)}%</span>
                    {/if}
                </div>
                {#if download}
                    <div
                        class="progress"
                        role="progressbar"
                        aria-valuemin="0"
                        aria-valuemax={download.bytes_total}
                        aria-valuenow={download.bytes_done}
                    >
                        <div
                            class="progress-fill"
                            style:width="{percentOf(download)}%"
                        ></div>
                    </div>
                {/if}
            </li>
        {/each}
    </ul>
{/if}

<style>
    .empty {
        margin: 0;
        font-size: var(--text-sm);
        color: var(--color-text-muted);
    }
    .video-list {
        list-style: none;
        margin: 0;
        padding: 0;
        display: flex;
        flex-direction: column;
        gap: var(--space-2);
    }
    .video-row {
        display: flex;
        flex-direction: column;
        gap: var(--space-1);
    }
    .video-row-main {
        display: flex;
        align-items: center;
        gap: var(--space-2);
        font-size: var(--text-sm);
    }
    .status-icon {
        display: inline-flex;
        flex-shrink: 0;
    }
    .status-icon :global(svg) {
        width: 0.9rem;
        height: 0.9rem;
    }
    .video-row.on-device .status-icon {
        color: var(--color-success);
    }
    .video-row.pending .status-icon {
        color: var(--color-warning);
    }
    .video-row.not-started .status-icon {
        color: var(--color-text-subtle);
    }
    .title {
        flex: 1;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }
    .video-row.not-started .title {
        color: var(--color-text-muted);
    }
    .percent {
        font-size: var(--text-xs);
        color: var(--color-text-muted);
        font-variant-numeric: tabular-nums;
        flex-shrink: 0;
    }
    .progress {
        width: 100%;
        height: 3px;
        background: var(--color-border);
        border-radius: var(--radius-full);
        overflow: hidden;
    }
    .progress-fill {
        height: 100%;
        background: var(--color-warning);
        transition: width var(--duration-fast) var(--ease-out);
    }
</style>
