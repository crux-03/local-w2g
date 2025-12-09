<script lang="ts">
    import { createEventDispatcher } from "svelte";

    export let videoId: string;
    export let filename: string;
    export let size: string;
    export let isCurrent: boolean = false;
    export let showDownload: boolean = false;

    const dispatch = createEventDispatcher();

    function handlePlay() {
        dispatch("play");
    }

    function handleDownload() {
        dispatch("download", { filename });
    }
</script>

<button
    class="playlist-item"
    class:current={isCurrent}
    on:click={handlePlay}
    type="button"
>
    <span class="file-name">{filename}</span>
    <span class="file-size">{size}</span>
    {#if showDownload}
        <div
            class="download-btn"
            on:click={(e) => {
                e.stopPropagation();
                handleDownload();
            }}
            role="button"
            tabindex="0"
            on:keydown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                    e.stopPropagation();
                    handleDownload();
                }
            }}
        >
            Download
        </div>
    {/if}
</button>

<style>
    .playlist-item {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
        padding: 0.75rem;
        background-color: var(--bg-tertiary);
        border: 1px solid var(--border-color);
        border-radius: 4px;
        margin-bottom: 0.5rem;
        cursor: pointer;
    }

    .playlist-item.current {
        border-color: var(--accent-green);
    }

    .file-name {
        font-size: 0.875rem;
        font-weight: 500;
    }

    .file-size {
        font-size: 0.75rem;
        color: var(--text-secondary);
    }

    .download-btn {
        margin-top: 0.5rem;
        width: 100%;
        font-size: 0.75rem;
        padding: 0.375rem;
        border-radius: 6px;
        border: 1px solid var(--border-color);
        font-family: inherit;
        color: var(--text-primary);
        background-color: var(--bg-tertiary);
        cursor: pointer;
        font-weight: 500;
        transition: all 0.2s;
    }

    .download-btn:hover {
        border-color: var(--accent-blue);
        background-color: var(--bg-secondary);
    }
</style>
