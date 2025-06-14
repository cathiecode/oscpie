import { buildVRChatCliCommand } from "./packages/vrchat-api/build.ts";

async function main() {
  try {
    await Promise.all([
      await new Deno.Command("cargo", {args: ["build", "--release"]}).spawn().status,
      await buildVRChatCliCommand.spawn().status,
    ]);
  } catch(e) {
    console.error("Failed to build:", e);
    Deno.exit(1);
  }
}

await main();
