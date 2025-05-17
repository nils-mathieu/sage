const std = @import("std");

/// Builds the project.
pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    //
    // Top-level steps.
    //
    const check_step = b.step("check", "Ensures that the Sage engine compiles");
    const test_step = b.step("test", "Runs the engine's integrated tests");

    //
    // `sage_core`
    //
    const sage_core_mod = b.addModule("sage_core", .{
        .root_source_file = b.path("libs/sage_core/sage_core.zig"),
        .target = target,
        .optimize = optimize,
    });
    const sage_core_test = addTestWithCustomRunner(b, sage_core_mod);
    check_step.dependOn(&sage_core_test.step);
    test_step.dependOn(&b.addRunArtifact(sage_core_test).step);

    //
    // `sage_input`
    //
    const sage_input_mod = b.addModule("sage_input", .{
        .root_source_file = b.path("libs/sage_input/sage_input.zig"),
        .target = target,
        .optimize = optimize,
    });
    const sage_input_test = addTestWithCustomRunner(b, sage_input_mod);
    check_step.dependOn(&sage_input_test.step);
    test_step.dependOn(&b.addRunArtifact(sage_input_test).step);

    //
    // `sage_window`
    //
    const sage_window_mod = b.addModule("sage_window", .{
        .root_source_file = b.path("libs/sage_window/sage_window.zig"),
        .target = target,
        .optimize = optimize,
    });
    sage_window_mod.addImport("sage_input", sage_input_mod);
    sage_window_mod.addImport("sage_core", sage_core_mod);
    const sage_window_test = addTestWithCustomRunner(b, sage_window_mod);
    check_step.dependOn(&sage_window_test.step);
    test_step.dependOn(&b.addRunArtifact(sage_window_test).step);

    //
    // `sage`
    //
    const sage_mod = b.addModule("sage", .{
        .root_source_file = b.path("libs/sage/sage.zig"),
        .target = target,
        .optimize = optimize,
    });
    sage_mod.addImport("sage_window", sage_window_mod);
    sage_mod.addImport("sage_core", sage_core_mod);
    sage_mod.addImport("sage_input", sage_input_mod);
    const sage_test = addTestWithCustomRunner(b, sage_mod);
    check_step.dependOn(&sage_test.step);
    test_step.dependOn(&b.addRunArtifact(sage_test).step);

    //
    // `sage_editor`
    //
    const sage_editor_exe = b.addExecutable(.{
        .name = "sage_editor",
        .root_module = b.createModule(.{
            .root_source_file = b.path("bins/sage_editor/main.zig"),
            .target = target,
            .optimize = optimize,
        }),
    });
    sage_editor_exe.root_module.addImport("sage", sage_mod);
    check_step.dependOn(&sage_editor_exe.step);
    const sage_editor_test = addTestWithCustomRunner(b, sage_editor_exe.root_module);
    check_step.dependOn(&sage_editor_test.step);
    test_step.dependOn(&b.addRunArtifact(sage_editor_test).step);
    b.step("run", "Runs the Sage editor").dependOn(&b.addRunArtifact(sage_editor_exe).step);
}

/// Creates a `Compile` test runner to test the provided
/// module.
///
/// The runne uses the custom test runner.
fn addTestWithCustomRunner(b: *std.Build, module: *std.Build.Module) *std.Build.Step.Compile {
    return b.addTest(.{
        .root_module = module,
        .test_runner = .{
            .path = b.path("test_runner.zig"),
            .mode = .simple,
        },
    });
}
