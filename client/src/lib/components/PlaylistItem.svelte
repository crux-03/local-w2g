<script lang="ts">
    import { createEventDispatcher } from "svelte";

    // svelte-ignore export_let_unused
    export let videoId: string;
    export let filename: string;
    export let size: string;
    export let isCurrent: boolean = false;
    export let isDownloaded: boolean = false;
    export let isDownloading: boolean = false;
    export let downloadProgress: number = 0; // 0-100

    const dispatch = createEventDispatcher();

    function handleClick() {
        dispatch("play");
    }
</script>

<button
    class="playlist-item"
    class:current={isCurrent}
    class:downloaded={isDownloaded}
    class:downloading={isDownloading}
    onclick={handleClick}
    style="--progress: {downloadProgress}%"
>
    <div class="progress-bar"></div>
    <div class="content">
        <div class="item-header">
            <span class="file-name">{filename}</span>
            {#if isDownloading}
                <span class="status-badge downloading">
                    ⏳ {downloadProgress}%
                </span>
            {:else if isDownloaded}
                <span class="status-badge downloaded">✓ Ready</span>
            {:else}
                <span class="status-badge pending">⬇ Download</span>
            {/if}
        </div>
        <span class="file-size">{size}</span>
    </div>
</button>

<style>
    .playlist-item {
        position: relative;
        display: flex;
        flex-direction: column;
        gap: 0.375rem;
        padding: 0.75rem;
        width: 100%;
        text-align: left;
        background-color: var(--bg-tertiary);
        border: 2px solid var(--border-color);
        border-radius: 4px;
        margin-bottom: 0.5rem;
        cursor: pointer;
        transition: all 0.2s;
        font-family: inherit;
        color: inherit;
        overflow: hidden;
    }

    .progress-bar {
        position: absolute;
        top: 0;
        left: 0;
        height: 100%;
        width: var(--progress);
        background: linear-gradient(
            90deg,
            rgba(33, 150, 243, 0.15) 0%,
            rgba(33, 150, 243, 0.25) 100%
        );
        transition: width 0.3s ease;
        z-index: 0;
    }

    .content {
        position: relative;
        z-index: 1;
        display: flex;
        flex-direction: column;
        gap: 0.375rem;
    }

    .playlist-item:hover:not(.downloading) {
        border-color: var(--accent-blue);
        background-color: var(--bg-secondary);
        transform: translateY(-1px);
    }

    .playlist-item:active:not(.downloading) {
        transform: translateY(0);
    }

    .playlist-item.current {
        border-color: var(--accent-green);
        background-color: rgba(76, 175, 80, 0.1);
    }

    .playlist-item.current .progress-bar {
        background: linear-gradient(
            90deg,
            rgba(76, 175, 80, 0.15) 0%,
            rgba(76, 175, 80, 0.25) 100%
        );
    }

    .playlist-item.downloading {
        opacity: 0.95;
        cursor: wait;
        border-color: var(--accent-blue);
    }

    .playlist-item.downloading .progress-bar {
        background: linear-gradient(
            90deg,
            rgba(33, 150, 243, 0.3) 0%,
            rgba(33, 150, 243, 0.4) 100%
        );
    }

    .item-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        gap: 0.5rem;
    }

    .file-name {
        font-size: 0.875rem;
        font-weight: 500;
        flex: 1;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .file-size {
        font-size: 0.75rem;
        color: var(--text-secondary);
    }

    .status-badge {
        font-size: 0.7rem;
        padding: 0.25rem 0.6rem;
        border-radius: 12px;
        font-weight: 600;
        white-space: nowrap;
        text-transform: uppercase;
        letter-spacing: 0.5px;
    }

    .status-badge.downloaded {
        background-color: #1b5e20;
        color: #4caf50;
    }

    .status-badge.pending {
        background-color: #1565c0;
        color: #90caf9;
    }

    .status-badge.downloading {
        background-color: #e65100;
        color: #ffb74d;
    }
</style>
