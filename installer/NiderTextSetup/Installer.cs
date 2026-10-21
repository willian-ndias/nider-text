using System;
using System.Diagnostics;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Reflection;
using System.Runtime.InteropServices;
using Microsoft.Win32;

namespace NiderTextSetup;

/// <summary>Instala por usuário (sem pedir administrador) em %LocalAppData%\Programs\Nider Text.</summary>
public static class Installer
{
    const string RegKey = @"Software\Microsoft\Windows\CurrentVersion\Uninstall\NiderText";
    const string ExeName = "wordcraft.exe";
    const string SetupName = "NiderTextSetup.exe";

    public static string InstallDir =>
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Programs", "Nider Text");

    static string StartMenuLink =>
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.Programs), "Nider Text.lnk");

    static string PinnedLink => Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
        "Microsoft", "Internet Explorer", "Quick Launch", "User Pinned", "TaskBar", "Nider Text.lnk");

    [DllImport("shell32.dll")]
    static extern void SHChangeNotify(int eventId, uint flags, IntPtr item1, IntPtr item2);

    /// <summary>Avisa o Windows que os ícones mudaram (senão a barra de tarefas e o menu Iniciar ficam com o ícone antigo em cache).</summary>
    static void RefreshShellIcons()
    {
        try { SHChangeNotify(0x08000000 /* SHCNE_ASSOCCHANGED */, 0 /* SHCNF_IDLIST */, IntPtr.Zero, IntPtr.Zero); }
        catch { }
    }

    static string DesktopLink =>
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory), "Nider Text.lnk");

    public static string CurrentVersion
    {
        get
        {
            var v = Assembly.GetExecutingAssembly().GetName().Version;
            return v == null ? "0.0.0" : $"{v.Major}.{v.Minor}.{v.Build}";
        }
    }

    public static string? InstalledVersion()
    {
        using var k = Registry.CurrentUser.OpenSubKey(RegKey);
        return k?.GetValue("DisplayVersion") as string;
    }

    public static string AppExe => Path.Combine(InstallDir, ExeName);

    /// <summary>True se há um Nider Text aberto a partir da pasta indicada.</summary>
    public static bool IsRunning(string dir)
    {
        foreach (var p in Process.GetProcessesByName("wordcraft"))
        {
            try
            {
                var path = p.MainModule?.FileName ?? "";
                if (path.StartsWith(dir, StringComparison.OrdinalIgnoreCase)) return true;
            }
            catch { }
        }
        return false;
    }

    static void ExtractPayload(string dir)
    {
        using var s = Assembly.GetExecutingAssembly().GetManifestResourceStream("payload.zip")
            ?? throw new InvalidOperationException("O instalador está incompleto (sem os arquivos do programa).");
        using var zip = new ZipArchive(s, ZipArchiveMode.Read);
        var root = Path.GetFullPath(dir) + Path.DirectorySeparatorChar;
        // O .zip tem uma pasta na raiz (wordcraft-x.y.z-...-portable/): tira esse prefixo.
        var exe = zip.Entries.FirstOrDefault(e =>
                {
                    var n = e.FullName.Replace('\\', '/');
                    return n == ExeName || n.EndsWith("/" + ExeName);
                })
            ?? throw new InvalidOperationException("wordcraft.exe não foi encontrado nos arquivos do instalador.");
        var full = exe.FullName.Replace('\\', '/');
        var prefix = full.Substring(0, full.Length - ExeName.Length);
        Directory.CreateDirectory(dir);
        foreach (var e in zip.Entries)
        {
            var name = e.FullName.Replace('\\', '/');
            if (!name.StartsWith(prefix) || name.EndsWith("/")) continue;
            var dest = Path.GetFullPath(Path.Combine(dir, name.Substring(prefix.Length)));
            if (!dest.StartsWith(root, StringComparison.OrdinalIgnoreCase)) continue;
            Directory.CreateDirectory(Path.GetDirectoryName(dest)!);
            e.ExtractToFile(dest, true);
        }
    }

    static void CreateShortcut(string link, string target, string workDir)
    {
        var type = Type.GetTypeFromProgID("WScript.Shell") ?? throw new InvalidOperationException("WScript.Shell indisponível.");
        dynamic shell = Activator.CreateInstance(type)!;
        dynamic sc = shell.CreateShortcut(link);
        sc.TargetPath = target;
        sc.WorkingDirectory = workDir;
        sc.Description = "Nider Text: editor de texto e diagramação de livros";
        sc.IconLocation = target + ",0";
        sc.Save();
    }

    public static void Install(bool desktopShortcut)
    {
        var dir = InstallDir;
        if (IsRunning(dir)) throw new InvalidOperationException("O Nider Text está aberto. Feche-o e tente de novo.");
        ExtractPayload(dir);

        var me = Environment.ProcessPath;
        var setupCopy = Path.Combine(dir, SetupName);
        if (me != null && !string.Equals(Path.GetFullPath(me), Path.GetFullPath(setupCopy), StringComparison.OrdinalIgnoreCase))
            File.Copy(me, setupCopy, true);

        CreateShortcut(StartMenuLink, AppExe, dir);
        if (desktopShortcut) CreateShortcut(DesktopLink, AppExe, dir);
        else if (File.Exists(DesktopLink)) File.Delete(DesktopLink);
        // Atalho fixado na barra de tarefas: recria para o Windows pegar o ícone novo.
        if (File.Exists(PinnedLink)) CreateShortcut(PinnedLink, AppExe, dir);
        RefreshShellIcons();

        using var k = Registry.CurrentUser.CreateSubKey(RegKey);
        k.SetValue("DisplayName", "Nider Text");
        k.SetValue("DisplayVersion", CurrentVersion);
        k.SetValue("Publisher", "Nider");
        k.SetValue("InstallLocation", dir);
        k.SetValue("DisplayIcon", AppExe + ",0");
        k.SetValue("UninstallString", "\"" + setupCopy + "\" /uninstall");
        k.SetValue("URLInfoAbout", "https://nider.com.br");
        k.SetValue("NoModify", 1, RegistryValueKind.DWord);
        k.SetValue("NoRepair", 1, RegistryValueKind.DWord);
    }

    public static void Uninstall()
    {
        var dir = InstallDir;
        if (IsRunning(dir)) throw new InvalidOperationException("O Nider Text está aberto. Feche-o e tente de novo.");
        if (File.Exists(StartMenuLink)) File.Delete(StartMenuLink);
        if (File.Exists(DesktopLink)) File.Delete(DesktopLink);
        Registry.CurrentUser.DeleteSubKeyTree(RegKey, false);
        // O desinstalador roda de dentro da pasta: apaga a pasta depois que ele fechar.
        Process.Start(new ProcessStartInfo("cmd.exe", "/c ping 127.0.0.1 -n 3 >nul & rmdir /s /q \"" + dir + "\"")
        {
            CreateNoWindow = true,
            UseShellExecute = false,
        });
    }

    /// <summary>"Usar sem instalar": extrai numa pasta temporária e abre.</summary>
    public static void RunWithoutInstalling()
    {
        var dir = Path.Combine(Path.GetTempPath(), "NiderText-" + CurrentVersion);
        if (!IsRunning(dir)) ExtractPayload(dir);
        Process.Start(new ProcessStartInfo(Path.Combine(dir, ExeName)) { UseShellExecute = true, WorkingDirectory = dir });
    }

    public static void Launch()
    {
        Process.Start(new ProcessStartInfo(AppExe) { UseShellExecute = true, WorkingDirectory = InstallDir });
    }
}
