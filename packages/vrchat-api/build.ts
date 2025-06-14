export const buildVRChatCliCommand = new Deno.Command("deno", {
  args: [
    "compile",
    "--allow-net",
    "--output", import.meta.dirname! + "/vrchatcli.exe",
    import.meta.dirname! + "/vrchatcli.ts"
  ],
});

if (import.meta.main) {
  try {
    await buildVRChatCliCommand.spawn().status;
  } catch (e) {
    console.error("Failed to build VRChat CLI:", e);
    Deno.exit(1);
  }
}
