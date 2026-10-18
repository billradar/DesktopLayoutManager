using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Windowing;
using Microsoft.UI;
using Windows.Storage.Pickers;
using Windows.Graphics;
using WinRT.Interop;

namespace DesktopLayoutManager.WinUI;

public sealed partial class MainWindow : Window
{
    private Snapshot? _loadedSnapshot;
    private string? _loadedPath;
    private string? _lastFillBackup;
    private bool _activated;

    public MainWindow()
    {
        InitializeComponent();
        Title = "桌面布局管理器";
        var windowId = Win32Interop.GetWindowIdFromWindow(WindowNative.GetWindowHandle(this));
        AppWindow.GetFromWindowId(windowId).Resize(new SizeInt32(1180, 760));
        ModeCombo.SelectedIndex = 0;
        IconList.ItemsSource = Array.Empty<DisplayRow>();
        Activated += MainWindow_Activated;
    }

    private async void MainWindow_Activated(object sender, WindowActivatedEventArgs args)
    {
        if (_activated)
            return;
        _activated = true;
        await RunUiAsync(async () => await RefreshAsync());
    }

    private async void Refresh_Click(object sender, RoutedEventArgs e) => await RunUiAsync(RefreshAsync);
    private async void SaveSnapshot_Click(object sender, RoutedEventArgs e) => await RunUiAsync(SaveSnapshotAsync);
    private async void OpenSnapshot_Click(object sender, RoutedEventArgs e) => await RunUiAsync(OpenSnapshotAsync);
    private async void PreviewSnapshot_Click(object sender, RoutedEventArgs e) => await RunUiAsync(PreviewSnapshotAsync);
    private async void Restore_Click(object sender, RoutedEventArgs e) => await RunUiAsync(RestoreAsync);
    private async void PreviewFill_Click(object sender, RoutedEventArgs e) => await RunUiAsync(PreviewFillAsync);
    private async void ExecuteFill_Click(object sender, RoutedEventArgs e) => await RunUiAsync(() => ExecuteFillAsync(null));
    private async void UndoFill_Click(object sender, RoutedEventArgs e) => await RunUiAsync(UndoFillAsync);
    private async void Export_Click(object sender, RoutedEventArgs e) => await RunUiAsync(ExportAsync);
    private async void ExecuteMode_Click(object sender, RoutedEventArgs e) => await RunUiAsync(ExecuteModeAsync);

    private async Task ExecuteModeAsync()
    {
        switch (ModeCombo.SelectedIndex)
        {
            case 0:
                await SaveSnapshotAsync();
                break;
            case 1:
                await ExecuteFillAsync(null);
                break;
            case 2:
                await SaveThenFillAsync();
                break;
            default:
                await ShowMessageAsync("请选择一种操作模式。", "尚未选择模式");
                break;
        }
    }

    private async Task RefreshAsync()
    {
        var reply = RustBridge.Call<SnapshotReply>("read");
        ShowCurrent(reply.Snapshot);
        SetStatus($"当前桌面：{reply.Snapshot.Icons.Count} 个项目；坐标为 Explorer 桌面视图坐标。");
        await Task.CompletedTask;
    }

    private async Task SaveSnapshotAsync()
    {
        var path = await PickSavePathAsync("desktop-layout.json", csv: false);
        if (path is null)
            return;

        if (IsLoadedPath(path))
        {
            await ShowMessageAsync("不能覆盖当前已打开的快照。请另选一个文件名。", "不能覆盖当前快照");
            return;
        }

        var reply = RustBridge.Call<SnapshotReply>("save_snapshot", new { path });
        _loadedSnapshot = reply.Snapshot;
        _loadedPath = reply.Path ?? path;
        ShowCurrent(reply.Snapshot);
        SetStatus($"已保存 {reply.Snapshot.Icons.Count} 个桌面项目：{_loadedPath}");
        await ShowMessageAsync($"已保存 {reply.Snapshot.Icons.Count} 个桌面项目的位置。\n\n{_loadedPath}", "快照已保存");
    }

    private async Task SaveThenFillAsync()
    {
        var path = await PickSavePathAsync($"desktop-layout-before-fill-{DateTimeOffset.Now:yyyyMMdd-HHmmss}.json", csv: false);
        if (path is null)
            return;
        if (IsLoadedPath(path))
        {
            await ShowMessageAsync("补位前快照不能覆盖当前已打开的布局文件。请另选一个文件名。", "不能覆盖当前快照");
            return;
        }

        var reply = RustBridge.Call<SnapshotReply>("save_snapshot", new { path });
        _loadedSnapshot = reply.Snapshot;
        _loadedPath = reply.Path ?? path;
        ShowCurrent(reply.Snapshot);
        SetStatus($"已保存补位前快照：{_loadedPath}；正在准备补位预览。");
        await ExecuteFillAsync(reply.Snapshot, _loadedPath);
    }

    private async Task OpenSnapshotAsync()
    {
        var path = await PickOpenJsonAsync();
        if (path is null)
            return;
        var reply = RustBridge.Call<SnapshotReply>("load_snapshot", new { path });
        _loadedSnapshot = reply.Snapshot;
        _loadedPath = reply.Path ?? path;
        SetStatus($"已载入快照：{Path.GetFileName(_loadedPath)}，{_loadedSnapshot.Icons.Count} 个项目。");
        await PreviewSnapshotAsync();
    }

    private async Task PreviewSnapshotAsync()
    {
        if (_loadedSnapshot is null)
        {
            await ShowMessageAsync("请先打开一个快照，或先保存当前布局。", "尚未载入快照");
            return;
        }

        var reply = RustBridge.Call<SnapshotReply>("read");
        var (rows, changed, missing, extra) = BuildSnapshotRows(_loadedSnapshot, reply.Snapshot);
        ShowRows(rows);
        var label = _loadedPath is null ? "快照" : Path.GetFileName(_loadedPath);
        SetStatus($"快照：{label} | 位置不同 {changed}，缺失或名称不唯一 {missing}，新增/未记录 {extra}。预览不会移动图标。");
        await Task.CompletedTask;
    }

    private async Task PreviewFillAsync()
    {
        var plan = RustBridge.Call<FillPlanReply>("preview_fill");
        ShowFillPlan(plan);
        SetStatus($"补位预览：将移动 {plan.Moves.Count} 个快捷方式；系统图标、文件和文件夹保持原位。预览不会修改桌面。");
        await ShowMessageAsync(
            $"预览完成，将按列优先顺序移动 {plan.Moves.Count} 个快捷方式。\n\n仅移动可可靠识别的快捷方式；真实文件、文件夹和系统图标保持原位。确认无误后选择“执行补位”。",
            "补位预览");
    }

    private async Task ExecuteFillAsync(Snapshot? preSavedSnapshot, string? preSavedPath = null)
    {
        var plan = RustBridge.Call<FillPlanReply>("preview_fill");
        ShowFillPlan(plan);
        if (preSavedSnapshot is not null && !SameLayout(preSavedSnapshot, plan.Snapshot))
        {
            SetStatus("保存快照后桌面发生变化；已停止补位，请重新保存快照并预览。");
            await ShowMessageAsync("保存补位前快照后，桌面布局发生了变化。本次补位已停止，请重新执行“快照 + 补位”。", "桌面状态已变化");
            return;
        }
        if (plan.Moves.Count == 0)
        {
            SetStatus("没有需要补位的图标。");
            await ShowMessageAsync(
                preSavedPath is null ? "当前没有需要补位的快捷方式。" : $"当前没有需要补位的快捷方式。\n\n补位前快照已保存：\n{preSavedPath}",
                "无需操作");
            return;
        }

        var message = $"即将按列优先顺序（每列从上到下，再从左到右）移动 {plan.Moves.Count} 个快捷方式。\n\n执行前会自动保存当前布局备份。真实文件、文件夹、系统图标和未识别项目不会移动。";
        if (preSavedPath is not null)
            message += $"\n\n补位前快照：\n{preSavedPath}";
        if (!await ConfirmAsync(message, "确认手动补位"))
        {
            SetStatus("已取消补位；桌面未移动。");
            return;
        }

        var result = RustBridge.Call<OperationReply>("execute_fill", new
        {
            expectedSnapshot = plan.Snapshot,
            expectedMoves = plan.Moves
        });
        _lastFillBackup = result.BackupPath;
        var latest = RustBridge.Call<SnapshotReply>("read").Snapshot;
        ShowCurrent(latest);
        SetStatus(result.BackupPath is null
            ? "没有需要补位的图标。"
            : $"补位完成，移动 {result.Moves.Count} 个快捷方式。撤销备份：{result.BackupPath}");
        await ShowMessageAsync(
            result.BackupPath is null
                ? "没有需要补位的快捷方式。"
                : $"补位完成，移动了 {result.Moves.Count} 个快捷方式。\n\n撤销备份：\n{result.BackupPath}\n\n也可通过“打开快照”恢复此备份。",
            result.BackupPath is null ? "无需操作" : "补位完成");
    }

    private async Task RestoreAsync()
    {
        if (_loadedSnapshot is null)
        {
            await ShowMessageAsync("请先打开一个快照。", "尚未载入快照");
            return;
        }

        var plan = RustBridge.Call<RestorePlanReply>("preview_restore", new { savedSnapshot = _loadedSnapshot });
        ShowRestorePlan(plan);
        var label = _loadedPath is null ? "当前快照" : _loadedPath;
        var prompt = $"将恢复 {plan.Moves.Count} 个唯一匹配且已可靠识别的快捷方式。执行前会自动备份当前布局。\n\n快照：{label}\n\n是否继续？";
        if (!await ConfirmAsync(prompt, "确认恢复布局"))
            return;

        if (plan.Skipped.Count > 0)
        {
            var sample = string.Join("\n", plan.Skipped.Take(8));
            var detail = $"快照中有 {plan.Skipped.Count} 个项目缺失、名称不唯一、属于受保护类型或超出安全范围。本次会跳过这些项目，只移动唯一匹配且已识别为快捷方式的图标。\n\n{sample}";
            if (plan.Skipped.Count > 8)
                detail += "\n…";
            if (!await ConfirmAsync(detail, "快照与当前桌面不一致", "继续恢复"))
                return;
        }

        if (plan.Moves.Count == 0)
        {
            SetStatus("没有可恢复的匹配图标。");
            await ShowMessageAsync("没有可恢复的唯一匹配快捷方式。", "无需操作");
            return;
        }

        var result = RustBridge.Call<OperationReply>("execute_restore", new
        {
            savedSnapshot = _loadedSnapshot,
            expectedSnapshot = plan.Snapshot,
            expectedMoves = plan.Moves,
            expectedSkipped = plan.Skipped
        });
        var latest = RustBridge.Call<SnapshotReply>("read").Snapshot;
        ShowCurrent(latest);
        SetStatus($"已请求恢复 {result.Moves.Count} 个图标。恢复前备份：{result.BackupPath}");
        await ShowMessageAsync($"已对 {result.Moves.Count} 个图标发送位置恢复请求。\n\n恢复前备份：\n{result.BackupPath}\n\n请检查桌面布局。", "恢复请求已完成");
    }

    private async Task UndoFillAsync()
    {
        if (string.IsNullOrWhiteSpace(_lastFillBackup) || !File.Exists(_lastFillBackup))
        {
            await ShowMessageAsync(
                "当前会话没有可直接撤销的补位备份。你可以打开 %LOCALAPPDATA%\\DesktopLayoutManager\\Backups 中的 before-fill-*.json，再执行“恢复布局”。",
                "没有撤销备份");
            return;
        }

        var plan = RustBridge.Call<RestorePlanReply>("preview_undo", new { path = _lastFillBackup });
        if (plan.Skipped.Count > 0)
        {
            throw new InvalidOperationException($"无法唯一匹配以下快捷方式：{string.Join("、", plan.Skipped)}");
        }
        ShowRestorePlan(plan);
        if (plan.Moves.Count == 0)
        {
            SetStatus("没有需要撤销的快捷方式位置。");
            return;
        }
        if (!await ConfirmAsync($"将恢复补位前布局中的 {plan.Moves.Count} 个快捷方式位置。\n\n备份：{_lastFillBackup}\n\n继续？", "撤销上次补位"))
            return;

        var saved = plan.SavedSnapshot ?? RustBridge.Call<SnapshotReply>("load_snapshot", new { path = _lastFillBackup }).Snapshot;
        var result = RustBridge.Call<OperationReply>("execute_undo", new
        {
            savedSnapshot = saved,
            expectedSnapshot = plan.Snapshot,
            expectedMoves = plan.Moves,
            expectedSkipped = plan.Skipped
        });
        _lastFillBackup = null;
        ShowCurrent(RustBridge.Call<SnapshotReply>("read").Snapshot);
        SetStatus($"已请求撤销补位并恢复 {result.Moves.Count} 个图标。撤销前备份：{result.BackupPath}");
        await ShowMessageAsync($"已请求恢复 {result.Moves.Count} 个图标的位置。\n\n撤销前备份：\n{result.BackupPath}\n\n请检查桌面布局。", "撤销请求已完成");
    }

    private async Task ExportAsync()
    {
        var path = await PickSavePathAsync("desktop-icons.json", csv: true);
        if (path is null)
            return;
        var result = RustBridge.Call<ExportReply>("export", new { path });
        SetStatus($"已导出 {result.Count} 个项目：{result.Path}");
    }

    private async Task<string?> PickOpenJsonAsync()
    {
        var picker = new FileOpenPicker();
        picker.FileTypeFilter.Add(".json");
        InitializeWithWindow.Initialize(picker, WindowNative.GetWindowHandle(this));
        return (await picker.PickSingleFileAsync())?.Path;
    }

    private async Task<string?> PickSavePathAsync(string suggestedName, bool csv)
    {
        var picker = new FileSavePicker { SuggestedFileName = Path.GetFileNameWithoutExtension(suggestedName) };
        picker.FileTypeChoices.Add("JSON 布局文件", new List<string> { ".json" });
        if (csv)
            picker.FileTypeChoices.Add("CSV 文件", new List<string> { ".csv" });
        InitializeWithWindow.Initialize(picker, WindowNative.GetWindowHandle(this));
        return (await picker.PickSaveFileAsync())?.Path;
    }

    private async Task<bool> ConfirmAsync(string content, string title, string primaryText = "继续")
    {
        var dialog = new ContentDialog
        {
            XamlRoot = Content.XamlRoot,
            Title = title,
            Content = content,
            PrimaryButtonText = primaryText,
            CloseButtonText = "取消",
            DefaultButton = ContentDialogButton.Close
        };
        return await dialog.ShowAsync() == ContentDialogResult.Primary;
    }

    private async Task ShowMessageAsync(string content, string title)
    {
        var dialog = new ContentDialog
        {
            XamlRoot = Content.XamlRoot,
            Title = title,
            Content = content,
            CloseButtonText = "确定",
            DefaultButton = ContentDialogButton.Close
        };
        await dialog.ShowAsync();
    }

    private async Task RunUiAsync(Func<Task> action)
    {
        try
        {
            await action();
        }
        catch (Exception error)
        {
            SetStatus("操作失败。");
            await ShowMessageAsync(error.Message, "操作失败");
        }
    }

    private void ShowCurrent(Snapshot snapshot) => ShowRows(snapshot.Icons.Select(icon => new DisplayRow(
        $"当前 #{icon.OriginalIndex}", icon.Name, Position(icon.X, icon.Y), "—")).ToList());

    private void ShowFillPlan(FillPlanReply plan)
    {
        var rows = plan.Snapshot.Icons.Select(icon =>
        {
            var move = plan.Moves.FirstOrDefault(item => item.Index == icon.OriginalIndex);
            var status = move is not null ? "将移动" : icon.Kind switch
            {
                ItemKind.Shortcut => "保持原位",
                ItemKind.System => "系统图标（保留）",
                ItemKind.File or ItemKind.Folder => "文件或文件夹（保留）",
                _ => "未识别（保留）"
            };
            return new DisplayRow(status, icon.Name, Position(icon.X, icon.Y), move is null ? Position(icon.X, icon.Y) : Position(move.ToX, move.ToY));
        }).ToList();
        ShowRows(rows);
    }

    private void ShowRestorePlan(RestorePlanReply plan)
    {
        var moveByIndex = plan.Moves.ToDictionary(move => move.Index);
        var rows = plan.Snapshot.Icons.Select(icon =>
        {
            if (moveByIndex.TryGetValue(icon.OriginalIndex, out var move))
                return new DisplayRow("将恢复", icon.Name, Position(icon.X, icon.Y), Position(move.ToX, move.ToY));
            return new DisplayRow(icon.Kind == ItemKind.Shortcut ? "保持原位" : "受保护（保留）", icon.Name, Position(icon.X, icon.Y), Position(icon.X, icon.Y));
        }).ToList();
        ShowRows(rows);
    }

    private static (List<DisplayRow> Rows, int Changed, int Missing, int Extra) BuildSnapshotRows(Snapshot saved, Snapshot current)
    {
        var rows = new List<DisplayRow>();
        var used = new HashSet<int>();
        var changed = 0;
        var missing = 0;
        foreach (var target in saved.Icons)
        {
            var matches = current.Icons.Where(icon => string.Equals(icon.Name, target.Name, StringComparison.OrdinalIgnoreCase)).ToArray();
            if (matches.Length == 1)
            {
                var icon = matches[0];
                used.Add(icon.OriginalIndex);
                var differs = icon.X != target.X || icon.Y != target.Y;
                if (differs) changed++;
                rows.Add(new DisplayRow(differs ? "位置不同" : "位置一致", target.Name, Position(icon.X, icon.Y), Position(target.X, target.Y)));
            }
            else
            {
                missing++;
                rows.Add(new DisplayRow(matches.Length == 0 ? "当前缺失" : "名称不唯一", target.Name, "—", Position(target.X, target.Y)));
            }
        }
        var extras = current.Icons.Where(icon => !used.Contains(icon.OriginalIndex)).ToArray();
        rows.AddRange(extras.Select(icon => new DisplayRow("快照中不存在", icon.Name, Position(icon.X, icon.Y), "—")));
        return (rows, changed, missing, extras.Length);
    }

    private void ShowRows(IReadOnlyList<DisplayRow> rows) => IconList.ItemsSource = rows;
    private void SetStatus(string message) => StatusText.Text = message;
    private static string Position(int x, int y) => $"({x}, {y})";

    private bool IsLoadedPath(string path)
    {
        if (string.IsNullOrWhiteSpace(_loadedPath))
            return false;
        return string.Equals(Path.GetFullPath(_loadedPath), Path.GetFullPath(path), StringComparison.OrdinalIgnoreCase);
    }

    private static bool SameLayout(Snapshot left, Snapshot right) =>
        left.Machine == right.Machine
        && left.ScreenWidth == right.ScreenWidth
        && left.ScreenHeight == right.ScreenHeight
        && left.Icons.SequenceEqual(right.Icons, IconComparer.Instance);

    private sealed class IconComparer : IEqualityComparer<IconItem>
    {
        public static IconComparer Instance { get; } = new();
        public bool Equals(IconItem? left, IconItem? right) => left is not null && right is not null
            && left.OriginalIndex == right.OriginalIndex && left.Name == right.Name
            && left.X == right.X && left.Y == right.Y && left.Kind == right.Kind;
        public int GetHashCode(IconItem icon) => HashCode.Combine(icon.OriginalIndex, icon.Name, icon.X, icon.Y, icon.Kind);
    }
}
