using System;
using System.Linq;
using System.Windows;

namespace NiderTextSetup;

public partial class App : Application
{
    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);
        if (e.Args.Any(a => a.Equals("/uninstall", StringComparison.OrdinalIgnoreCase)))
        {
            var ok = MessageBox.Show("Remover o Nider Text deste computador?\n\nOs seus documentos não são apagados.",
                "Desinstalar o Nider Text", MessageBoxButton.YesNo, MessageBoxImage.Question);
            if (ok == MessageBoxResult.Yes)
            {
                try
                {
                    Installer.Uninstall();
                    MessageBox.Show("O Nider Text foi removido.", "Nider Text", MessageBoxButton.OK, MessageBoxImage.Information);
                }
                catch (Exception ex)
                {
                    MessageBox.Show("Não foi possível remover tudo: " + ex.Message, "Nider Text", MessageBoxButton.OK, MessageBoxImage.Warning);
                }
            }
            Shutdown();
            return;
        }
        new MainWindow().Show();
    }
}
