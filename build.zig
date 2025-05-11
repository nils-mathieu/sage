const std = @import("std");

pub fn build(b: *std.Build) void {
    //
    // Initialization and parameters for the project.
    //

    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    //
    // Create the top-level steps.
    //

    const test_step = b.step("test", "run unit tests for the whole project");
    const check_step = b.step("check", "check whether the project compiles");

    //
    // `sage_core`
    //

    const sage_core_module = b.addModule("sage_core", .{
        .target = target,
        .optimize = optimize,
        .root_source_file = b.path("libs/sage_core/sage_core.zig"),
    });

    const sage_core_test = b.addTest(.{ .root_module = sage_core_module });
    test_step.dependOn(&b.addRunArtifact(sage_core_test).step);
    check_step.dependOn(&sage_core_test.step);
}
