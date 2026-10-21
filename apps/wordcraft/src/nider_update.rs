//! Nider: ao abrir, pergunta ao site Nider se há versão nova do Nider Text e mostra um aviso.
//!
//! A consulta roda numa thread à parte (não trava a abertura) com o `curl` do sistema, que vem
//! no Windows 10 e 11: assim o app não ganha dependência nova de rede. Sem internet, ou com
//! qualquer resposta estranha, simplesmente não aparece nada.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

const URL: &str = "https://nider.com.br/api/apps/nider-text/versao";
const PAGINA: &str = "https://nider.com.br/downloads";

#[derive(Clone)]
struct Nova {
    versao: String,
    notas: String,
    pagina: String,
    /// Link direto do instalador novo (o site manda quando a atualização é pública).
    download: Option<String>,
}

static ESTADO: OnceLock<Mutex<Option<Nova>>> = OnceLock::new();
static FECHADO: AtomicBool = AtomicBool::new(false);

/// Começa a consulta em segundo plano.
pub fn iniciar(ctx: egui::Context) {
    std::thread::spawn(move || {
        if let Some(nova) = consultar() {
            if let Ok(mut g) = ESTADO.get_or_init(|| Mutex::new(None)).lock() {
                *g = Some(nova);
            }
            ctx.request_repaint();
        }
    });
}

fn consultar() -> Option<Nova> {
    let mut cmd = std::process::Command::new("curl");
    cmd.args(["-s", "-f", "-m", "8", "-H", "Accept: application/json", URL]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: nenhuma janela de console pisca
    }
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let corpo = out.stdout.get(..out.stdout.len().min(1 << 20))?;
    let v: serde_json::Value = serde_json::from_slice(corpo).ok()?;
    let versao = v.get("versao")?.as_str()?.trim().to_string();
    if versao.is_empty() || versao.len() > 64 || !mais_nova(&versao, env!("CARGO_PKG_VERSION")) {
        return None;
    }
    let pagina = v
        .get("pagina")
        .and_then(|p| p.as_str())
        .filter(|p| p.starts_with("https://nider.com.br/") && !p.contains(['"', '\\', ' ']))
        .unwrap_or(PAGINA)
        .to_string();
    let download = v
        .get("download")
        .and_then(|d| d.as_str())
        .filter(|d| d.starts_with("https://nider.com.br/") && !d.contains(['"', '\\', ' ']))
        .map(str::to_string);
    let notas = v.get("notas").and_then(|n| n.as_str()).unwrap_or("").chars().take(600).collect();
    Some(Nova { versao, notas, pagina, download })
}

/// `0.1.5-alfa` → (0, 1, 5). Sufixos (-alfa, +build) não contam; o que não for número dá None.
fn numeros(v: &str) -> Option<(u64, u64, u64)> {
    let core = v.trim().trim_start_matches(['v', 'V']).split(['-', '+']).next()?;
    let mut partes = core.split('.').map(|p| p.parse::<u64>().ok());
    let a = partes.next()??;
    let b = partes.next().unwrap_or(Some(0))?;
    let c = partes.next().unwrap_or(Some(0))?;
    Some((a, b, c))
}

fn mais_nova(publicada: &str, atual: &str) -> bool {
    matches!((numeros(publicada), numeros(atual)), (Some(p), Some(a)) if p > a)
}

/// Desenha o aviso (a cada quadro) enquanto houver versão nova e a pessoa não tiver fechado.
pub fn mostrar(ctx: &egui::Context) {
    if FECHADO.load(Ordering::Relaxed) {
        return;
    }
    let Some(nova) = ESTADO.get().and_then(|m| m.lock().ok()).and_then(|g| g.clone()) else {
        return;
    };
    let mut aberto = true;
    let mut fechar = false;
    egui::Window::new("Nova versão do Nider Text")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::RIGHT_BOTTOM, [-16.0, -40.0])
        .open(&mut aberto)
        .show(ctx, |ui| {
            ui.label(format!("A versão {} está disponível.", nova.versao.trim_start_matches(['v', 'V'])));
            if !nova.notas.is_empty() {
                ui.add_space(4.0);
                ui.label(egui::RichText::new(&nova.notas).small().weak());
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                // Com o link direto, o navegador já baixa o instalador: é só abrir e clicar em Atualizar.
                let (rotulo, link) = match &nova.download {
                    Some(d) => ("Atualizar agora", d.as_str()),
                    None => ("Baixar a versão nova", nova.pagina.as_str()),
                };
                if ui.button(rotulo).clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(link));
                    fechar = true;
                }
                if ui.button("Depois").clicked() {
                    fechar = true;
                }
            });
        });
    if !aberto || fechar {
        FECHADO.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compara_versoes_ignorando_sufixo() {
        assert!(mais_nova("0.1.6-alfa", "0.1.5-alfa"));
        assert!(mais_nova("v0.2.0", "0.1.9"));
        assert!(!mais_nova("0.1.5-alfa", "0.1.5-alfa"));
        assert!(!mais_nova("0.1.4", "0.1.5"));
        assert!(!mais_nova("lixo", "0.1.5"));
    }
}
