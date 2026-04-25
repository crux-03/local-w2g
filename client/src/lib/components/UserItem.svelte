<script lang="ts">
    import {
        ArrowDownFromLine,
        ArrowUpFromLine,
        MessageSquare,
        Radio,
        TvMinimalPlay,
        Upload,
        UserCog,
    } from "@lucide/svelte";
    import type { User } from "$lib/api/user";
    import { userStore } from "$lib/stores/users.svelte";
    import { hasPermission } from "$lib/helpers/permission";
    import { Permissions } from "$lib/api/types";

    let { user }: { user: User } = $props();
    let me = $derived(userStore.me);

    let isSelf = $derived(user.id === userStore.me?.id);
    const hasManageUserPerms = $derived(
        me ? hasPermission(me.permissions, Permissions.MANAGE_USERS) : false,
    );
    let isExpanded = $state(false);

    type PermissionMeta = {
        bit: number;
        label: string;
        hint: string;
        icon: typeof UserCog;
    };

    const PERMISSION_META: PermissionMeta[] = [
        {
            bit: Permissions.MANAGE_PLAYBACK,
            label: "Playback",
            hint: "Play, pause, seek",
            icon: TvMinimalPlay,
        },
        {
            bit: Permissions.MANAGE_MEDIA,
            label: "Media",
            hint: "Add or remove videos",
            icon: Upload,
        },
        {
            bit: Permissions.SEND_MESSAGE,
            label: "Chat",
            hint: "Post in chat",
            icon: MessageSquare,
        },
        {
            bit: Permissions.SEND_STATE,
            label: "State",
            hint: "Broadcast own state",
            icon: Radio,
        },
        {
            bit: Permissions.MANAGE_USERS,
            label: "Users",
            hint: "Edit others' permissions",
            icon: UserCog,
        },
    ];

    async function togglePermission(bit: number) {
        const granted = (user.permissions & bit) === 0;
        await userStore.updatePermissions(user.id, bit, granted);
    }
</script>

<div class="viewer-card">
    <div class="header">
        <div class="username">
            <span class={isSelf ? "me" : ""}
                >{user.display_name ?? user.id}</span
            >
            {#if isSelf}
                <div class="badge">you</div>
            {/if}
        </div>
        <button
            class="btn btn-ghost util-btn align-end"
            onclick={() => (isExpanded = !isExpanded)}
        >
            {#if isExpanded}
                <ArrowUpFromLine />
            {:else}
                <ArrowDownFromLine />
            {/if}
        </button>
    </div>
    {#if isExpanded}
        <div class="details">
            <h6 class="section-title">Details</h6>
            boo
        </div>
        {#if hasManageUserPerms}
            <div class="permission-section">
                <h6 class="section-title">Permissions</h6>
                <div class="permission-toggles">
                    {#each PERMISSION_META as perm (perm.bit)}
                        {@const granted = (user.permissions & perm.bit) !== 0}
                        <button
                            type="button"
                            class="perm-toggle"
                            class:granted
                            disabled={isSelf}
                            title={perm.hint}
                            aria-pressed={granted}
                            onclick={() => togglePermission(perm.bit)}
                        >
                            <perm.icon aria-hidden="true" />
                            <span>{perm.label}</span>
                        </button>
                    {/each}
                </div>
            </div>
        {/if}
    {/if}
</div>

<style>
    .viewer-card {
        display: flex;
        flex-direction: column;
        background: var(--color-surface);
        border: var(--border-thin) solid var(--color-border);
        border-radius: var(--radius-md);
    }
    .header {
        display: flex;
        align-items: center;
        padding: var(--space-3);
    }
    .details {
        border-top: var(--border-thin) solid var(--color-border);
        padding: var(--space-3);
        font-size: var(--text-sm);
    }
    .viewer-card .me {
        font-weight: bold;
        color: var(--twilight-200);
    }
    .username {
        display: flex;
        gap: var(--space-3);
        align-items: baseline;
    }
    .badge {
        display: flex;
        flex-shrink: 1;
        border-radius: var(--radius-sm);
        background-color: var(--twilight-400);
        padding: var(--space-1) var(--space-2);
        font-size: var(--text-xs);
        box-shadow: var(--shadow-md);
    }
    .permission-section {
        display: flex;
        flex-direction: column;
        gap: var(--space-2);
        border-top: var(--border-thin) solid var(--color-border);
        padding: var(--space-3);
    }
    .section-title {
        margin: 0;
        font-size: var(--text-xs);
        text-transform: uppercase;
        letter-spacing: 0.05em;
        opacity: 0.7;
    }
    .permission-toggles {
        display: flex;
        flex-wrap: wrap;
        gap: var(--space-2);
    }
    .perm-toggle {
        display: inline-flex;
        align-items: center;
        gap: var(--space-2);
        padding: var(--space-1) var(--space-2);
        border: var(--border-thin) solid var(--color-border);
        border-radius: var(--radius-sm);
        background: transparent;
        color: inherit;
        font-size: var(--text-xs);
        cursor: pointer;
        transition:
            background-color 0.15s ease,
            border-color 0.15s ease;
    }
    .perm-toggle:hover:not(:disabled) {
        border-color: var(--twilight-300);
    }
    .perm-toggle.granted {
        background: var(--twilight-400);
        border-color: var(--twilight-300);
    }
    .perm-toggle:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }
    .perm-toggle :global(svg) {
        width: 0.9rem;
        height: 0.9rem;
        flex-shrink: 0;
    }
    .align-end {
        margin-left: auto;
    }
    .util-btn {
        max-width: 3em;
    }
    .util-btn > :global(svg) {
        width: 1.2rem;
        height: 1.2rem;
        flex-shrink: 0;
    }
</style>
