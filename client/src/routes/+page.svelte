<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { ClipboardPaste, Folder } from "@lucide/svelte";
    import { goto } from "$app/navigation";
    import { readText } from "@tauri-apps/plugin-clipboard-manager";
    import { onMount } from "svelte";
    import Toast from "$src/lib/components/Toast.svelte";
    import { addError } from "$src/lib/stores/error.svelte";

    let username = $state("");
    let server_url = $state("");
    let server_pw = $state("");

    let video_dir = $state("");
    let mpv_binary = $state("");

    onMount(async () => {
        username = await invoke("load_username");
        server_url = await invoke("load_server_url");
        mpv_binary = await invoke("load_mpv_binary");
        video_dir = await invoke("load_videos_dir");
    });

    async function connectClick() {
        try {
            console.log("connecting");
            await invoke("connect", {
                username: username,
                serverUrl: server_url,
                serverPw: server_pw,
            });
            goto("/app");
        } catch (error) {
            addError(`Failed to connect: ${error}`);
        }
    }

    async function pastePassword() {
        try {
            const text = await readText();
            server_pw = text;
        } catch (error) {
            addError(`Error when reading clipboard: ${error}`);
        }
    }

    async function browseVideoDir() {
        try {
            const path = (await invoke("pick_folder")) as string;
            if (path) {
                video_dir = path;
                await invoke("set_videos_dir", { path: video_dir });
            }
        } catch (error) {
            addError(`Error when selecting video path: ${error}`);
        }
    }

    async function browseMpvBinary() {
        try {
            const path = (await invoke("pick_file")) as string;
            if (path) {
                mpv_binary = path;
                await invoke("set_mpv_binary", { path: mpv_binary });
            }
        } catch (error) {
            addError(`Error when selecting video path: ${error}`);
        }
    }
</script>

<main class="container">
    <div class="container-inner">
        <h1>Watch2Gether</h1>

        <section>
            <h2>Connection</h2>
            <div class="field">
                <label for="username-input" class="label">Username</label>
                <input
                    id="username-input"
                    class="input"
                    bind:value={username}
                />
            </div>
            <div class="field">
                <label for="server-url-input" class="label">Server URL</label>
                <input
                    id="server-url-input"
                    class="input"
                    bind:value={server_url}
                />
            </div>
            <div class="field">
                <label for="password-input" class="label">Password</label>
                <div class="field-row">
                    <input
                        id="password-input"
                        class="input"
                        type="password"
                        bind:value={server_pw}
                    />
                    <button class="btn btn-secondary" onclick={pastePassword}
                        ><ClipboardPaste /></button
                    >
                </div>
            </div>
        </section>

        <section>
            <h2>Settings</h2>
            <div class="field">
                <label for="video-dir-input" class="label"
                    >Video Directory</label
                >
                <div class="field-row">
                    <input
                        id="video-dir-input"
                        class="input"
                        bind:value={video_dir}
                        readonly
                    />
                    <button class="btn btn-secondary" onclick={browseVideoDir}
                        ><Folder /></button
                    >
                </div>
            </div>
            <div class="field">
                <label for="mpv-binary-input" class="label">MPV Binary</label>
                <div class="field-row">
                    <input
                        id="mpv-binary-input"
                        class="input"
                        bind:value={mpv_binary}
                        readonly
                    />
                    <button class="btn btn-secondary" onclick={browseMpvBinary}
                        ><Folder /></button
                    >
                </div>
            </div>
        </section>
        <div class="form-actions">
            <button class="btn btn-primary" onclick={connectClick}
                >Connect</button
            >
        </div>
    </div>
</main>

<Toast />

<style>
    .container {
        display: flex;
        justify-content: center;
        min-height: 100%;
        padding: var(--space-8) var(--space-4);
        background-color: var(--color-bg);
    }

    .container-inner {
        width: 100%;
        max-width: 560px;
    }

    h1 {
        margin: 0 0 var(--space-8);
    }

    section {
        display: flex;
        flex-direction: column;
        gap: var(--space-3);
        margin-bottom: var(--space-8);
    }

    section:last-of-type {
        margin-bottom: 0;
    }

    section h2 {
        margin: 0 0 var(--space-1);
    }

    .field-row {
        display: flex;
        gap: var(--space-2);
    }

    .field-row .input {
        flex: 1;
    }
    .form-actions {
        margin-top: var(--space-8);
    }
    .form-actions .btn {
        width: 100%;
    }
</style>
