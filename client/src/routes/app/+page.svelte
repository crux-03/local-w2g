<script lang="ts">
    import type { MessageGroup } from "$src/lib/api/message";
    import {
        Permissions,
        type Entry,
        type EntryKindChat,
        type EntryKindWidget,
        type WidgetStateDownload,
        type WidgetStateUpload,
    } from "$src/lib/api/types";
    import EntryRouter from "$src/lib/components/chat/EntryRouter.svelte";
    import PlaybackControls from "$src/lib/components/controls/PlaybackControls.svelte";
    import Details from "$src/lib/components/Details.svelte";
    import PlaylistItem from "$src/lib/components/PlaylistItem.svelte";
    import UserItem from "$src/lib/components/UserItem.svelte";
    import { hasPermission } from "$src/lib/helpers/permission";
    import { messageStore } from "$src/lib/stores/messages.svelte";
    import { playlistStore } from "$src/lib/stores/playlist.svelte";
    import { userStore } from "$src/lib/stores/users.svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { onMount } from "svelte";

    let chatboxContent = $state("");

    let me = $derived(userStore.me);

    const hasManageMediakPerms = $derived(
        me ? hasPermission(me.permissions, Permissions.MANAGE_MEDIA) : false,
    );

    let groupedMessages = $derived.by(() => {
        const groups: MessageGroup[] = [];

        messageStore.messages.forEach((msg) => {
            const lastGroup = groups[groups.length - 1];

            // Determine if this message "belongs" to the previous group
            const canGroup =
                lastGroup &&
                lastGroup.type === msg.kind.type &&
                getSenderId(lastGroup.entries[0]) === getSenderId(msg);

            if (canGroup) {
                lastGroup.entries.push(msg);
            } else {
                groups.push({
                    user: getSenderId(msg),
                    type: msg.kind.type,
                    entries: [msg],
                });
            }
        });

        return groups;
    });

    // Helper to extract uploader or sender ID
    function getSenderId(entry: Entry): string {
        if (entry.kind.type === "widget") {
            const kind = entry.kind as EntryKindWidget;
            if (kind.state.kind === "download") {
                return (kind.state as WidgetStateDownload).reporter;
            }
            if (kind.state.kind === "upload") {
                return (kind.state as WidgetStateUpload).uploader;
            }
        }
        if (entry.kind.type === "chat") {
            return (entry.kind as EntryKindChat).sender;
        }
        return "0"; // 0 = system
    }

    const handleKeyDown = (event: KeyboardEvent) => {
        if (event.key === "Enter") {
            const trimmed = chatboxContent.trim();

            if (!trimmed) return;

            event.preventDefault();
            messageStore.sendMessage(trimmed);
            chatboxContent = "";
        }
    };

    onMount(async () => {
        await playlistStore.init();
        await userStore.requestUsers();
        await userStore.identifySelf();
        await messageStore.requestMessageHistory();
        await playlistStore.requestPlaylist();
        await playlistStore.loadLocalFiles();
    });

    async function uploadVideo() {
        try {
            const path = (await invoke("pick_file", {
                filters: [["Video", ["mp4", "mkv", "webm", "mov"]]],
            })) as string;
            if (path) {
                await invoke("upload_video", { filePath: path });
            }
        } catch (error) {
            console.log(`Error when uploading file: ${error}`);
        }
    }
</script>

<main class="app">
    <!-- LEFT: viewers + playback -->
    <aside class="col">
        <h6 class="section-title">Viewers</h6>
        <div class="viewers">
            {#each userStore.users as user (user.id)}
                <UserItem {user} />
            {/each}
        </div>
        <PlaybackControls />
    </aside>

    <!-- MIDDLE: status + chat -->
    <section class="col">
        <Details />
        <div class="chat">
            <div class="chat-messages">
                {#each messageStore.messages as message (message.id)}
                    <EntryRouter {message} />
                {/each}
            </div>
            <div class="chat-input">
                <input
                    class="input"
                    placeholder="Message"
                    bind:value={chatboxContent}
                    onkeydown={handleKeyDown}
                />
            </div>
        </div>
    </section>

    <!-- RIGHT: playlist + add -->
    <aside class="col">
        <h6 class="section-title">Playlist</h6>
        <div class="playlist">
            {#each playlistStore.entries as entry (entry.order)}
                <PlaylistItem {entry} />
            {/each}
        </div>
        {#if hasManageMediakPerms}
            <button class="btn btn-secondary" onclick={uploadVideo}
                >Add videos</button
            >
        {/if}
    </aside>
</main>

<style>
    .app {
        display: grid;
        grid-template-columns: 260px minmax(0, 1fr) 300px;
        gap: var(--space-3);
        box-sizing: border-box;
        height: 100%;
        padding: var(--space-3);
        background: var(--color-bg);
    }

    .col {
        display: flex;
        flex-direction: column;
        gap: var(--space-3);
        min-height: 0;
    }

    .section-title {
        margin: 0;
    }

    /* LEFT */
    .viewers {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: var(--space-2);
        min-height: 0;
        overflow-y: auto;
    }

    /* MIDDLE */
    .chat {
        flex: 1;
        display: flex;
        flex-direction: column;
        min-height: 0;
        background: var(--color-surface);
        border: var(--border-thin) solid var(--color-border);
        border-radius: var(--radius-lg);
        overflow: hidden;
    }

    .chat-messages {
        flex: 1;
        display: flex;
        flex-direction: column-reverse;
        min-height: 0;
        padding: var(--space-3);
        overflow-y: auto;
        gap: var(--space-1);
    }

    .chat-input {
        padding: var(--space-2);
        border-top: var(--border-thin) solid var(--color-border);
    }

    /* RIGHT */
    .playlist {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: var(--space-1);
        min-height: 0;
        overflow-y: auto;
    }
</style>
