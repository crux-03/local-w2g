<script lang="ts">
    import type { VideoEntry } from "$lib/api/video";
    import {
        ChevronDown,
        ChevronUp,
        Download,
        ListVideo,
        Pencil,
    } from "@lucide/svelte";
    import { userStore } from "../stores/users.svelte";
    import { hasPermission } from "../helpers/permission";
    import { Permissions } from "../api/types";
    import { playlistStore } from "../stores/playlist.svelte";
    import { invoke } from "@tauri-apps/api/core";

    let { entry }: { entry: VideoEntry } = $props();
    let me = $derived(userStore.me);

    const isFirst = $derived(entry.order === 0);
    const isLast = $derived(playlistStore.isLastItem(entry.id));
    const isSelected = $derived(playlistStore.isSelected(entry.id));

    const isDownloaded = $derived(playlistStore.fileOnDevice(entry.id));
    const hasManageMediaPerms = $derived(
        me ? hasPermission(me.permissions, Permissions.MANAGE_MEDIA) : false,
    );

    async function handleDownload() {
        try {
            await invoke("download_file", {
                videoId: entry.id,
                displayName: entry.display_name,
            });
        } catch (error) {
            console.log(`Error when downloading: ${error}`);
        }
    }

    async function selectVideo() {
        await playlistStore.selectVideo(entry.id);
    }
</script>

<div class="card {isSelected ? 'selected' : ''}">
    <div class="body">
        <span
            class="display_name {isSelected ? 'selected' : ''}"
            title={String(entry.display_name)}>{entry.display_name}</span
        >

        {#if hasManageMediaPerms}
            <button class="btn btn-ghost util-btn"><Pencil /></button>
        {:else if !isDownloaded}
            <button
                class="btn btn-download util-btn align-end"
                onclick={handleDownload}><Download /></button
            >
        {/if}
    </div>

    {#if hasManageMediaPerms}
        <div class="toolbar">
            <button class="btn btn-ghost util-btn" disabled={isFirst}
                ><ChevronUp /></button
            >
            <button class="btn btn-ghost util-btn" disabled={isLast}
                ><ChevronDown /></button
            >
            {#if !isSelected}
                <button
                    class="btn btn-select util-btn align-end"
                    onclick={selectVideo}><ListVideo /></button
                >
            {/if}
            {#if !isDownloaded}
                <button
                    class="btn btn-download util-btn {isSelected
                        ? 'align-end'
                        : ''}"
                    onclick={handleDownload}><Download /></button
                >
            {/if}
        </div>
    {/if}
</div>

<style>
    .card {
        display: flex;
        flex-direction: column;
        width: 100%;
        min-width: 0;
        padding: var(--space-2) var(--space-4);
        border-radius: var(--radius-md);
    }
    .card.selected {
        background-color: var(--jade-500);
    }
    .body {
        display: flex;
        flex-direction: row;
        align-items: center;
        gap: var(--space-2);
        padding: var(--space-1) 0px;
        min-height: 3em;
    }
    .display_name {
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        flex: 1;
        min-width: 0;
    }
    .display_name.selected {
        color: var(--twilight-200);
        font-weight: bold;
    }
    .toolbar {
        display: flex;
        flex-direction: row;
        border-top: var(--border-thin) solid var(--color-border);
        gap: var(--space-1);
        padding: var(--space-1) 0px;
    }
    .util-btn {
        max-width: 3em;
    }
    .util-btn > :global(svg) {
        width: 1.2rem;
        height: 1.2rem;
        flex-shrink: 0;
    }
    .align-end {
        margin-left: auto;
    }
    .btn-download {
        position: relative;
        overflow: hidden;
        background-color: var(--sage-500);
    }
    .btn-download:hover {
        background-color: var(--sage-600);
    }
    .btn-download::before {
        content: "";
        position: absolute;
        inset: 0 auto 0 0;
        width: calc(var(--progress, 0) * 100%);
        background-color: var(--sage-700); /* darker shade of the same hue */
        transition: width 150ms linear;
    }
    .btn-download > :global(svg) {
        position: relative; /* keep the icon above the fill */
    }
    .btn-select {
        position: relative;
        overflow: hidden;
        background-color: var(--twilight-400);
    }
    .btn-select:hover {
        background-color: var(--twilight-500);
    }
</style>
