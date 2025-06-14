import { z } from "https://deno.land/x/zod@v3.16.1/mod.ts";

// Inputマッピング (e.g., "click": { output: "/actions/..." })
export const InputMappingSchema = z.record(
    z.object({
        output: z.string(),
    }),
);

// sourceの定義
export const SourceSchema = z.object({
    inputs: InputMappingSchema,
    mode: z.string(),
    path: z.string(),
    parameters: z
        .record(z.string())
        .optional(), // parametersが存在する場合
});

// haptics、poses、skeletonなどの簡易マッピング用
export const OutputPathMappingSchema = z.object({
    output: z.string(),
    path: z.string(),
});

// action set keyに対応するbindingエントリ
export const ActionBindingSchema = z.object({
    sources: z.array(SourceSchema).optional(),
    chords: z.array(z.any()).optional(), // 中身の仕様が不明なため any
    haptics: z.array(OutputPathMappingSchema).optional(),
    poses: z.array(OutputPathMappingSchema).optional(),
    skeleton: z.array(OutputPathMappingSchema).optional(),
});

// bindingsフィールド全体
export const BindingsSchema = z.record(ActionBindingSchema);

export const BindingFileSchema = z.object({
    bindings: BindingsSchema,
    controller_type: z.string(),
    description: z.string(),
    name: z.string(),
});

export const DefaultBindingSchema = z.object({
    controller_type: z.string(),
    binding_url: z.string(),
});

export const ActionSchema = z.object({
    name: z.string(),
    requirement: z.enum(["mandatory", "optional", "suggested"]).optional(),
    type: z.enum([
        "boolean",
        "vibration",
        "vector1",
        "vector2",
        "vector3",
        "pose",
        "skeleton",
    ]),
    skeleton: z.string().optional(),
});

export const ActionSetSchema = z.object({
    name: z.string(),
    usage: z.enum(["leftright", "single"]),
});

export const LocalizationSchema = z.object({
    language_tag: z.string(),
}).catchall(z.string()); // NOTE: catchall for Record<ActionId, LocalizedString>

export const ActionManifestSchema = z.object({
    default_bindings: z.array(DefaultBindingSchema),
    actions: z.array(ActionSchema),
    action_sets: z.array(ActionSetSchema),
    localization: z.array(LocalizationSchema),
});
