<script lang="ts">
    import "$lib/assets/styles/main.css";
    import { goto } from "$app/navigation";
    import { invoke } from "@tauri-apps/api/core";

    let server_url = $state("");
    let server_pw = $state("");

    async function connect() {
        if (!server_url.trim()) {
            console.error("Please enter a server URL");
            return;
        }

        if (!server_pw.trim()) {
            console.error("Please enter a server URL");
            return;
        }

        try {
            await invoke("connect", {
                serverUrl: server_url,
                serverPw: server_pw,
            });
            goto("/app");
        } catch (error) {
            console.error("Failed to connect:", error);
        }
    }
</script>

<main class="container">
    <div>
        <h style="padding: 5px;">Server URL</h>
        <input bind:value={server_url} />
    </div>
    <div>
        <h style="padding: 5px;">Password</h>
        <input bind:value={server_pw} />
    </div>
    <div style="padding-top: 10px;">
        <button onclick={connect}>Connect</button>
    </div>
</main>

<style>
    .container {
        margin: 0;
        padding-top: 10vh;
        display: flex;
        flex-direction: column;
        justify-content: center;
        text-align: center;
    }

    h1 {
        text-align: center;
    }
</style>
