import { parseArgs } from "jsr:@std/cli/parse-args";
import * as types from "./action_manifest_types.ts";
import { z } from "https://deno.land/x/zod@v3.16.1/index.ts";

const ACTION_TYPE_MAP = {
    "boolean": "BooleanInput",
    "vector1": "Vector1Input",
    "vector2": "Vector2Input",
    "vector3": "Vector3Input",
    "vibration": "BooleanOutput",
    "pose": "PoseInput",
    "skeleton": "SkeletonOutput",
} as const;

const INPUT_ACTIONS = [
    "boolean",
    "vector1",
    "vector2",
    "vector3",
    "pose",
    "skeleton",
] as const;

const VECTOR_ACTIONS = [
    "vector1",
    "vector2",
    "vector3",
] as const;

type ActionType = keyof typeof ACTION_TYPE_MAP;
type InputActionType = typeof INPUT_ACTIONS[number];
type VectorActionType = typeof VECTOR_ACTIONS[number];

const inputActionFunctionByType: Record<
    InputActionType,
    (actionName: string) => string
> = {
    boolean: (actionName) => `
        pub fn get_${actionName}(
            &self,
        ) -> Result<BooleanInput> {
            self.get_digital_action_data(self.generated_fields.action_handle_${actionName})
        }
    `,
    vector1: (actionName) => `
        pub fn get_${actionName}(
            &self,
        ) -> Result<Vector1Input> {
            self.get_vector1_action_data(self.generated_fields.action_handle_${actionName})
        }
    `,
    vector2: (actionName) => `
        pub fn get_${actionName}(
            &self,
        ) -> Result<Vector2Input> {
            self.get_vector2_action_data(self.generated_fields.action_handle_${actionName})
        }
    `,
    vector3: (actionName) => `
        pub fn get_${actionName}(
            &self,
        ) -> Result<Vector3Input> {
            self.get_vector3_action_data(self.generated_fields.action_handle_${actionName})
        }
    `,
    pose: (actionName) => `
        pub fn get_${actionName}(
            &self,
            tracking_universe_origin: TrackingUniverseOrigin
        ) -> Result<PoseInput> {
            self.get_pose_action_data(tracking_universe_origin, self.generated_fields.action_handle_${actionName})
        }
    `,
    skeleton: (actionName) => `
        pub fn get_${actionName}(
            &self,
        ) -> Result<SkeletonOutput> {
            todo!();
        }
    `,
};

const flatCode = (code: string) => {
    return code
        .split("\n")
        .map((line) => line.trim())
        .filter((line) => line.length > 0)
        .join("\n");
};

function isInputAction(actionType: ActionType): actionType is InputActionType {
    return INPUT_ACTIONS.includes(actionType as InputActionType);
}

/*const inputActionFunctionSignature = (
    actionName: string,
    actionType: InputActionType,
) => {
    const functionName = `get_${canonicalizedActionName(actionName)}`;

    return `
        pub fn ${functionName}(&self) -> ${ACTION_TYPE_MAP[actionType]}
    `.trim();
};*/

const inputActionFunction = (actionName: string, actionType: ActionType) => {
    if (!(isInputAction(actionType))) {
        throw new Error(`Invalid action type: ${actionType}`);
    }

    return inputActionFunctionByType[actionType](
        canonicalizedActionName(actionName),
    );
};

const initFunction = (
    actionManifest: z.infer<typeof types.ActionManifestSchema>,
) => {
    return "";
    const actionSets = actionManifest.action_sets.map((actionSet) => `
        // Action Set: ${actionSet.name}
        self.action_sets
    `).join("\n");

    return `
    pub fn init(&mut self) -> Self {
        ${actionSets}
    }
    `;
};

const inputActionFunctions = (
    actionManifest: z.infer<typeof types.ActionManifestSchema>,
) => {
    return actionManifest.actions
        .filter((action) => isInputAction(action.type))
        .map((action) => inputActionFunction(action.name, action.type))
        .join("\n\n");
};

const activationFunctions = (
    actionManifest: z.infer<
        typeof types.ActionManifestSchema
    >,
) => {
    return actionManifest.action_sets.map((actionSet) => {
        return `
            pub fn activate_${
            canonicalizedActionSetName(actionSet.name)
        }(&mut self) {
                self.activate_action_set(self.generated_fields.action_set_handle_${
            canonicalizedActionSetName(actionSet.name)
        })
            }

            pub fn deactivate_${
            canonicalizedActionSetName(actionSet.name)
        }(&mut self) {
                // Deactivation logic for ${actionSet.name}
                self.deactivate_action_set(self.generated_fields.action_set_handle_${
            canonicalizedActionSetName(actionSet.name)
        })
            }
        `;
    }).join("\n");
};

const canonicalizedActionName = (actionName: string) => {
    return actionName.replaceAll("/", "_").replace(/^_/, "");
};

const canonicalizedActionSetName = (actionSetName: string) => {
    return actionSetName.replaceAll("/", "_").replace(/^_/, "");
};

const fieldGenerationFunction = (
    actionManifest: z.infer<typeof types.ActionManifestSchema>,
) => {
    const actionDeclarations = actionManifest.actions.map((action) => `
            let action_handle_${
        canonicalizedActionName(action.name)
    } = Self::get_action_handle(sys, "${action.name}")?;
        `).join("\n");

    const actionFields = actionManifest.actions.map((action) => `
            action_handle_${canonicalizedActionName(action.name)},
        `).join("\n");

    const actionSetDeclarations = actionManifest.action_sets.map(
        (actionSet) => `
            let action_set_handle_${
            canonicalizedActionSetName(actionSet.name)
        } = Self::get_action_set_handle(sys, "${actionSet.name}")?;
        `,
    ).join("\n");

    const actionSetFields = actionManifest.action_sets.map(
        (actionSet) => `
            action_set_handle_${canonicalizedActionSetName(actionSet.name)},
        `,
    ).join("\n");

    return `
        fn generate_fields(sys: &sys::VR_IVRInput_FnTable) -> Result<GeneratedFields> {
            ${actionDeclarations}
            ${actionSetDeclarations}

            Ok(GeneratedFields {
                ${actionFields}
                ${actionSetFields}
            })
        }
    `;
};

const generatedFields = (
    actionManifest: z.infer<typeof types.ActionManifestSchema>,
) => {
    const generatedActionFields = actionManifest.actions.map((action) => `
            action_handle_${
        canonicalizedActionName(action.name)
    }: sys::VRActionHandle_t,
        `).join("\n");

    const generatedActionSetFields = actionManifest.action_sets.map(
        (actionSet) => `
            action_set_handle_${
            canonicalizedActionSetName(actionSet.name)
        }: sys::VRActionSetHandle_t,
        `,
    ).join("\n");

    return `
        struct GeneratedFields {
            ${generatedActionFields}
            ${generatedActionSetFields}
        }
    `;
};

const code = (actionManifest: z.infer<typeof types.ActionManifestSchema>) => {
    return `
        impl Input {
            ${initFunction(actionManifest)}
            ${fieldGenerationFunction(actionManifest)}
            ${inputActionFunctions(actionManifest)}
            ${activationFunctions(actionManifest)}
        }

        ${generatedFields(actionManifest)}
    `.trim();
};

function main() {
    const flags = parseArgs(Deno.args);

    const manifestPath = flags._?.[0];
    const preludePath = flags._?.[1];
    const generatedPath = flags._?.[2];

    if (
        typeof manifestPath !== "string" || typeof generatedPath !== "string" ||
        typeof preludePath !== "string"
    ) {
        console.error(
            "Please provide the path to the action manifest file, the prelude file and the output file.",
        );
        Deno.exit(1);
    }

    const actionManifest = types.ActionManifestSchema.parse(
        JSON.parse(Deno.readTextFileSync(manifestPath)),
    );

    const generationNotice =
        `// This file is generated by utils/generate_action_codes.ts. Do not edit it manually.\n\n`;

    const preludeCode = Deno.readTextFileSync(preludePath).replace(
        /\/\/\s*STUB_FOLLOWS.*/s,
        "",
    );
    const generatedCode = code(actionManifest);

    Deno.writeTextFileSync(
        generatedPath,
        flatCode(generationNotice + preludeCode + generatedCode),
    );
}

if (import.meta.main) {
    main();
}
