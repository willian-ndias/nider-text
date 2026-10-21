using System;
using System.Diagnostics;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Navigation;

namespace NiderTextSetup;

public partial class MainWindow : Window
{
    bool done;

    public MainWindow()
    {
        InitializeComponent();
        Loaded += (_, _) => Refresh();
    }

    void Refresh()
    {
        var current = Installer.CurrentVersion;
        var installed = Installer.InstalledVersion();
        if (installed == null)
        {
            PrimaryBtn.Content = "Instalar";
            return;
        }
        InfoText.Visibility = Visibility.Visible;
        if (Version.TryParse(installed, out var a) && Version.TryParse(current, out var b) && a < b)
        {
            PrimaryBtn.Content = "Atualizar";
            InfoText.Text = $"Versão instalada: {installed}  ·  nova versão: {current}";
        }
        else
        {
            PrimaryBtn.Content = "Reinstalar";
            InfoText.Text = $"Versão instalada: {installed}  ·  este instalador: {current}";
        }
    }

    void SetBusy(bool busy)
    {
        PrimaryBtn.IsEnabled = !busy;
        PortableBtn.IsEnabled = !busy;
        Progress.Visibility = busy ? Visibility.Visible : Visibility.Collapsed;
    }

    async void Primary_Click(object sender, RoutedEventArgs e)
    {
        if (done)
        {
            Close();
            return;
        }
        var desktop = DesktopBox.IsChecked == true;
        var open = OpenBox.IsChecked == true;
        var updating = Equals(PrimaryBtn.Content, "Atualizar");
        StatusText.Text = updating ? "Atualizando…" : "Instalando…";
        SetBusy(true);
        try
        {
            await Task.Run(() => Installer.Install(desktop));
            SetBusy(false);
            if (open)
            {
                Installer.Launch();
                Close();
                return;
            }
            done = true;
            StatusText.Text = updating ? "Pronto! O Nider Text foi atualizado." : "Pronto! O Nider Text foi instalado. Procure por ele no menu Iniciar.";
            PrimaryBtn.Content = "Fechar";
            PortableBtn.Visibility = Visibility.Collapsed;
            Options.Visibility = Visibility.Collapsed;
        }
        catch (Exception ex)
        {
            SetBusy(false);
            StatusText.Text = "Não foi possível instalar: " + ex.Message;
        }
    }

    /// <summary>Atualização pedida pelo app: espera ele fechar (até 60 s), instala e reabre.</summary>
    public async void AutoUpdate()
    {
        Options.Visibility = Visibility.Collapsed;
        PortableBtn.Visibility = Visibility.Collapsed;
        SetBusy(true);
        StatusText.Text = "Esperando o Nider Text fechar…";
        for (var i = 0; i < 120 && Installer.IsRunning(Installer.InstallDir); i++) await Task.Delay(500);
        StatusText.Text = "Atualizando…";
        try
        {
            var desktop = System.IO.File.Exists(Installer.DesktopLink);
            await Task.Run(() => Installer.Install(desktop));
            StatusText.Text = "Pronto! Abrindo a versão nova…";
            Installer.Launch();
            await Task.Delay(800);
            Close();
        }
        catch (Exception ex)
        {
            SetBusy(false);
            PrimaryBtn.Content = "Atualizar";
            StatusText.Text = "Não foi possível atualizar: " + ex.Message;
        }
    }

    async void Portable_Click(object sender, RoutedEventArgs e)
    {
        StatusText.Text = "Abrindo sem instalar…";
        SetBusy(true);
        try
        {
            await Task.Run(Installer.RunWithoutInstalling);
            Close();
        }
        catch (Exception ex)
        {
            SetBusy(false);
            StatusText.Text = "Não foi possível abrir: " + ex.Message;
        }
    }

    void Link_Navigate(object sender, RequestNavigateEventArgs e)
    {
        try { Process.Start(new ProcessStartInfo(e.Uri.AbsoluteUri) { UseShellExecute = true }); } catch { }
        e.Handled = true;
    }
}
