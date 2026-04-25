<script lang="ts">
    import { Permissions } from "$lib/api/types";
    import { hasPermission } from "$lib/helpers/permission";
    import { userStore } from "$lib/stores/users.svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { playlistStore } from "$lib/stores/playlist.svelte";

    let me = $derived(userStore.me);
    let selected: boolean = $derived(playlistStore.selected !== undefined);

    const hasManagePlaybackPerms = $derived(
        me ? hasPermission(me.permissions, Permissions.MANAGE_PLAYBACK) : false,
    );

    async function handlePlay() {
        try {
            const activeId = playlistStore.selected;
            await invoke("play", {
                videoId: activeId,
            });
        } catch (error) {
            console.log(`Error when playing: ${error}`);
        }
    }
</script>

<div class="status">
    <h6 class="ping-text">30ms</h6>
    <div class="header">
        Selected:
        {#if selected && hasManagePlaybackPerms}
            <button
                class="btn btn-primary"
                style="margin-left: auto;"
                onclick={handlePlay}>Start</button
            >
        {/if}
    </div>
   
</div>

<style>
    .status {
        display: flex;
        flex-direction: column;
        justify-content: center;
        padding: var(--space-3) var(--space-4);
        background: var(--color-surface);
        border: var(--border-thin) solid var(--color-border);
        border-radius: var(--radius-md);
    }
    .header {
        display: flex;
        align-items: baseline;
    }
    .ping-text {
        display: flex;
        align-items: center;
        font-size: var(--text-xs);
        text-transform: lowercase;
        letter-spacing: 0.05em;
        opacity: 0.7;
    }
</style>
