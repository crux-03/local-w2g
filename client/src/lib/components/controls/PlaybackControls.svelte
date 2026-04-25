<script>
    import { Permissions } from "$src/lib/api/types";
    import { hasPermission } from "$src/lib/helpers/permission";
    import { userStore } from "$src/lib/stores/users.svelte";
    import {
        Pause,
        Play,
        RefreshCw,
        StepBack,
        StepForward,
    } from "@lucide/svelte";

    let me = $derived(userStore.me);

    const hasManagePlaybackPerms = $derived(
        me ? hasPermission(me.permissions, Permissions.MANAGE_PLAYBACK) : false,
    );

    async function handleSeekBack() {}
    async function handleSeekForward() {}
    async function handlePlay() {}
    async function handlePause() {}
    async function handleResync() {}
</script>

{#if hasManagePlaybackPerms}
    <div class="playback">
        <button
            class="btn btn-ghost"
            aria-label="Seek back 5 seconds"
            title="Seek -5s"
            onclick={handleSeekBack}><StepBack /></button
        >
        <button
            class="btn btn-primary"
            aria-label="Play"
            title="Play"
            onclick={handlePlay}><Play /></button
        >
        <button
            class="btn btn-ghost"
            aria-label="Pause"
            title="Pause"
            onclick={handlePause}><Pause /></button
        >
        <button
            class="btn btn-ghost"
            aria-label="Resync"
            title="Resync"
            onclick={handleResync}><RefreshCw /></button
        >
        <button
            class="btn btn-ghost"
            aria-label="Seek forward 5 seconds"
            title="Seek +5s"
            onclick={handleSeekForward}><StepForward /></button
        >
    </div>
{/if}

<style>
    .playback {
        display: flex;
        gap: var(--space-1);
        padding: var(--space-1);
        background: var(--color-surface);
        border: var(--border-thin) solid var(--color-border);
        border-radius: var(--radius-md);
    }

    .playback .btn {
        flex: 1;
        padding: var(--space-2);
    }
</style>
