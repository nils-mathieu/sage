const Uuid = @import("../Uuid.zig");
const std = @import("std");
const oom = @import("../utility/errors.zig").oom;

pub const ComponentA = struct {
    pub const component_uuid = Uuid.comptimeParse("7550d4ad-38a8-470a-958c-1a98a616b29b");

    data: u32,
};

pub const ComponentB = struct {
    pub const component_uuid = Uuid.comptimeParse("c4d2fefa-c2e3-4640-b9c2-bf19f5a43852");

    data: []const u8,
};

pub const ComponentC = struct {
    pub const component_uuid = Uuid.comptimeParse("f21d6bab-7caa-466d-9c25-c2d4c43585e0");

    data: std.ArrayListUnmanaged(u8),

    pub fn fromSlice(data: []const u8) ComponentC {
        var array_list = std.ArrayListUnmanaged(u8).empty;
        array_list.appendSlice(std.testing.allocator, data) catch oom();
        return ComponentC{ .data = array_list };
    }

    pub fn deinit(self: *ComponentC, allocator: std.mem.Allocator) void {
        self.data.deinit(allocator);
    }
};
