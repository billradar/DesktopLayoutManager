using System.Runtime.InteropServices;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace DesktopLayoutManager.WinUI;

internal static class RustBridge
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true,
        Converters = { new JsonStringEnumConverter(JsonNamingPolicy.SnakeCaseLower) }
    };

    [DllImport("desktop_layout_manager.dll", EntryPoint = "dlm_call", CallingConvention = CallingConvention.Cdecl)]
    private static extern nint CallNative(nint requestUtf8);

    [DllImport("desktop_layout_manager.dll", EntryPoint = "dlm_free_string", CallingConvention = CallingConvention.Cdecl)]
    private static extern void FreeNative(nint value);

    public static T Call<T>(string action, object? args = null)
    {
        var request = new Dictionary<string, object?> { ["action"] = action };
        if (args is not null)
        {
            foreach (var property in args.GetType().GetProperties())
            {
                request[property.Name] = property.GetValue(args);
            }
        }

        var json = JsonSerializer.Serialize(request, JsonOptions);
        var input = Marshal.StringToCoTaskMemUTF8(json);
        try
        {
            var responsePointer = CallNative(input);
            if (responsePointer == 0)
            {
                throw new InvalidOperationException("Rust 桌面操作库没有返回结果。");
            }

            try
            {
                var responseJson = Marshal.PtrToStringUTF8(responsePointer)
                    ?? throw new InvalidOperationException("无法读取 Rust 桌面操作库的响应。");
                using var document = JsonDocument.Parse(responseJson);
                var root = document.RootElement;
                if (!root.TryGetProperty("ok", out var ok) || !ok.GetBoolean())
                {
                    var message = root.TryGetProperty("error", out var error)
                        ? error.GetString()
                        : null;
                    throw new InvalidOperationException(message ?? "桌面操作失败。");
                }

                if (!root.TryGetProperty("data", out var data))
                {
                    throw new InvalidOperationException("Rust 桌面操作库的响应缺少数据。");
                }
                return data.Deserialize<T>(JsonOptions)
                    ?? throw new InvalidOperationException("无法解析 Rust 桌面操作库的响应数据。");
            }
            finally
            {
                FreeNative(responsePointer);
            }
        }
        finally
        {
            Marshal.FreeCoTaskMem(input);
        }
    }
}
