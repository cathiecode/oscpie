import * as path from "jsr:@std/path";
import { parseArgs } from "jsr:@std/cli/parse-args";
import { z } from "https://deno.land/x/zod@v3.16.1/mod.ts";

import {
    ActionManifestSchema,
    BindingFileSchema,
} from "./action_manifest_types.ts";

type Options = {
    "allow-missing-localization": boolean;
};

type ValidationResult<T> = {
    success: false;
} | {
    success: true;
    data: T;
};

const VALIDATION_FAILED = {
    success: false,
} as const;

function validateBindings(
    bindingPath: string,
): ValidationResult<z.infer<typeof BindingFileSchema>> {
    const binding = JSON.parse(Deno.readTextFileSync(bindingPath));

    if (!binding) {
        console.error(
            `Failed to read or parse the binding file at ${bindingPath}.`,
        );
        return VALIDATION_FAILED;
    }

    const parsedBinding = BindingFileSchema.safeParse(binding);

    if (!parsedBinding.success) {
        console.error(
            `Binding validation failed for ${bindingPath}:`,
            parsedBinding.error.errors,
        );
        return VALIDATION_FAILED;
    }

    return {
        success: true,
        data: parsedBinding.data,
    };
}

function validateManifest(
    manifestPath: string,
    options: Options,
): ValidationResult<z.infer<typeof ActionManifestSchema>> {
    const manifest = JSON.parse(Deno.readTextFileSync(manifestPath));

    if (!manifest) {
        console.error("Failed to read or parse the manifest file.");
        return VALIDATION_FAILED;
    }

    const parsedManifest = ActionManifestSchema.safeParse(manifest);

    if (!parsedManifest.success) {
        console.error(
            "Manifest validation failed:",
            parsedManifest.error.errors,
        );
        return VALIDATION_FAILED;
    }

    for (const localization of parsedManifest.data.localization) {
        const missingActions = new Set(
            parsedManifest.data.actions.map((action) => action.name),
        );

        for (
            const [actionId, _] of Object.entries(localization)
        ) {
            if (actionId === "language_tag") {
                continue; // Skip the language tag itself
            }

            missingActions.delete(actionId);
        }

        if (missingActions.size > 0) {
            console.error(
                `Localization is missing for actions: ${
                    Array.from(missingActions).join(", ")
                }`,
            );

            if (!options["allow-missing-localization"]) {
                return VALIDATION_FAILED;
            }
        }
    }

    for (const defaultBinding of parsedManifest.data.default_bindings) {
        const bindingPath = path.resolve(
            path.dirname(manifestPath),
            defaultBinding.binding_url,
        );

        // TODO: check if the all default bindings exist
        const parsedBindings = validateBindings(bindingPath);

        if (!parsedBindings.success) {
            console.error(`Binding validation failed for ${bindingPath}.`);
            return VALIDATION_FAILED;
        }

        // NOTE: This part is commented out because it does not work
        /*
        const missingActions = new Set(
            parsedManifest.data.actions.map((action) => action.name),
        );

        for (const actionBinding in parsedBindings.data.bindings) {
            missingActions.delete(actionBinding);
        }

        if (missingActions.size > 0) {
            console.error(
                `Default bindings of ${parsedBindings.data.controller_type} are missing for actions: ${
                    Array.from(missingActions).join(", ")
                }`,
            );

            return VALIDATION_FAILED;
        }
        */
    }

    return {
        success: true,
        data: parsedManifest.data,
    };
}

function main() {
    const flags = parseArgs(Deno.args, {
        boolean: ["allow-missing-localization"],
    });

    const manifestPath = flags._?.[0];

    if (typeof manifestPath !== "string") {
        console.error("Please provide the path to the action manifest file.");
        Deno.exit(1);
    }

    if (validateManifest(manifestPath, flags).success) {
        console.log("Manifest is valid.");
    } else {
        console.error("Manifest validation failed.");
        Deno.exit(1);
    }
}

if (import.meta.main) {
    main();
}
