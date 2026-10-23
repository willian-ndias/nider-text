//! Nider: as mesmas regras do Nider Etapas. Ao abrir, pergunta ao site Nider se há versão nova e,
//! havendo, o aviso pergunta Baixar / Depois. Com o programa aberto, olha de novo a cada 4 horas:
//! aí só acende o botão de atualização na barra de título (clicar nele pergunta de novo). Nada é
//! baixado antes do Baixar; durante o download a seta desce e mostra a porcentagem.
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
/// O aviso está fechado: começa fechado e só abre na consulta ao abrir o programa (ou pelo botão
/// da barra de título).
static FECHADO: AtomicBool = AtomicBool::new(true);
/// "Baixar" clicado: o instalador está sendo baixado.
static BAIXANDO: AtomicBool = AtomicBool::new(false);
/// Quanto do instalador já chegou, 0–100; acima disso, ainda não se sabe o tamanho.
static PORCENTO: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(u8::MAX);

/// Com o programa aberto, olha de novo a cada 4 horas (como no Nider Etapas).
const DE_NOVO: std::time::Duration = std::time::Duration::from_secs(4 * 60 * 60);

/// O último `Content-Length` do arquivo de cabeçalhos do curl (depois dos redirecionamentos).
fn tamanho(cabecalhos: &std::path::Path) -> Option<u64> {
    let texto = std::fs::read_to_string(cabecalhos).ok()?;
    texto
        .lines()
        .rev()
        .filter_map(|l| l.split_once(':'))
        .filter(|(k, _)| k.trim().eq_ignore_ascii_case("content-length"))
        .find_map(|(_, v)| v.trim().parse::<u64>().ok())
}
static ERRO: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn erro(msg: Option<String>) {
    if let Ok(mut g) = ERRO.get_or_init(|| Mutex::new(None)).lock() {
        *g = msg;
    }
}

/// Baixa o instalador novo para a pasta temporária e o abre em modo "/atualizar": ele espera
/// este app fechar, troca a versão instalada e reabre o Nider Text, como no Nider Etapas.
fn atualizar(ctx: egui::Context, versao: String, link: String) {
    BAIXANDO.store(true, Ordering::Relaxed);
    erro(None);
    std::thread::spawn(move || {
        let resultado = (|| -> Result<(), String> {
            let nome: String = versao.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-').take(40).collect();
            let destino = std::env::temp_dir().join(format!("nider-text-setup-{nome}.exe"));
            let cabecalhos = std::env::temp_dir().join(format!("nider-text-setup-{nome}.headers"));
            let _ = std::fs::remove_file(&destino);
            let mut cmd = std::process::Command::new("curl");
            cmd.args(["-s", "-f", "-L", "-m", "900", "-D"]).arg(&cabecalhos).arg("-o").arg(&destino).arg(&link);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x0800_0000);
            }
            let falhou = || "Não consegui baixar a versão nova. Confira a internet e tente de novo.".to_string();
            let mut filho = cmd.spawn().map_err(|_| falhou())?;
            // Enquanto baixa: a porcentagem, pelo tamanho do arquivo contra o que o site informou.
            let ok = loop {
                match filho.try_wait() {
                    Ok(Some(status)) => break status.success(),
                    Ok(None) => {}
                    Err(_) => break false,
                }
                if let (Some(total), Ok(meta)) = (tamanho(&cabecalhos), std::fs::metadata(&destino))
                    && total > 0
                {
                    let p = (meta.len().saturating_mul(100) / total).min(99);
                    PORCENTO.store(u8::try_from(p).unwrap_or(99), Ordering::Relaxed);
                    ctx.request_repaint();
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            };
            let _ = std::fs::remove_file(&cabecalhos);
            if !ok {
                return Err(falhou());
            }
            PORCENTO.store(100, Ordering::Relaxed);
            // Só um instalador do Windows (começa com "MZ") é aberto.
            let mut cabeca = [0u8; 2];
            let lido = std::fs::File::open(&destino).and_then(|mut f| std::io::Read::read_exact(&mut f, &mut cabeca));
            if lido.is_err() || &cabeca != b"MZ" {
                let _ = std::fs::remove_file(&destino);
                return Err("O arquivo baixado não é um instalador do Windows.".into());
            }
            std::process::Command::new(&destino).arg("/atualizar").spawn().map_err(|e| format!("Não consegui abrir o instalador: {e}"))?;
            Ok(())
        })();
        match resultado {
            // O instalador assume: fecha o app (se houver texto não salvo, o app pergunta antes).
            Ok(()) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Err(e) => {
                erro(Some(e));
                BAIXANDO.store(false, Ordering::Relaxed);
                PORCENTO.store(u8::MAX, Ordering::Relaxed);
            }
        }
        ctx.request_repaint();
    });
}

/// Começa as consultas em segundo plano: agora (o aviso pergunta) e a cada 4 horas (só o botão
/// da barra de título acende).
pub fn iniciar(ctx: egui::Context) {
    std::thread::spawn(move || {
        let mut primeira = true;
        loop {
            if !BAIXANDO.load(Ordering::Relaxed)
                && let Some(nova) = consultar()
            {
                if let Ok(mut g) = ESTADO.get_or_init(|| Mutex::new(None)).lock() {
                    *g = Some(nova);
                }
                if primeira {
                    FECHADO.store(false, Ordering::Relaxed);
                }
                ctx.request_repaint();
            }
            primeira = false;
            std::thread::sleep(DE_NOVO);
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

/// Seta para baixo que desce e some, em ciclo, enquanto o instalador baixa.
fn seta_baixando(ui: &mut egui::Ui, cor: egui::Color32) {
    wordcraft_ui_egui::chrome::down_arrow(ui, cor);
}

/// Desenha o aviso (a cada quadro) enquanto houver versão nova e a pessoa não tiver fechado, e
/// conta à barra de título o que mostrar no botão de atualização
/// (`wordcraft_ui_egui::chrome::UPDATE_BADGE`).
pub fn mostrar(ctx: &egui::Context) {
    let Some(nova) = ESTADO.get().and_then(|m| m.lock().ok()).and_then(|g| g.clone()) else {
        return;
    };
    let badge = wordcraft_ui_egui::chrome::UpdateBadge {
        versao: nova.versao.trim_start_matches(['v', 'V']).to_string(),
        baixando: BAIXANDO.load(Ordering::Relaxed),
        porcento: Some(PORCENTO.load(Ordering::Relaxed)).filter(|p| *p <= 100),
    };
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(wordcraft_ui_egui::chrome::UPDATE_BADGE), badge));
    // O botão da barra de título foi clicado: pergunta de novo.
    if ctx.data_mut(|d| d.remove_temp::<bool>(egui::Id::new(wordcraft_ui_egui::chrome::UPDATE_ASK))).unwrap_or(false) {
        FECHADO.store(false, Ordering::Relaxed);
    }
    if FECHADO.load(Ordering::Relaxed) {
        return;
    }
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
            if let Some(e) = ERRO.get().and_then(|m| m.lock().ok()).and_then(|g| g.clone()) {
                ui.label(egui::RichText::new(e).color(ui.visuals().error_fg_color));
                ui.add_space(4.0);
            }
            if BAIXANDO.load(Ordering::Relaxed) {
                ui.horizontal(|ui| {
                    let cor = ui.visuals().text_color();
                    seta_baixando(ui, cor);
                    let quanto = Some(PORCENTO.load(Ordering::Relaxed)).filter(|p| *p <= 100).map(|p| format!(" {p}%")).unwrap_or_default();
                    ui.label(format!("Baixando a versão nova{quanto}… o Nider Text vai fechar, atualizar e abrir de novo."));
                });
                return;
            }
            ui.horizontal(|ui| {
                match &nova.download {
                    // Baixa e atualiza sozinho, sem passar pelo navegador.
                    Some(d) => {
                        // Como no Nider Etapas: nada é baixado antes do Baixar.
                        if ui.button("Baixar").clicked() {
                            atualizar(ui.ctx().clone(), nova.versao.clone(), d.clone());
                        }
                    }
                    None => {
                        if ui.button("Baixar a versão nova").clicked() {
                            ui.ctx().open_url(egui::OpenUrl::new_tab(nova.pagina.as_str()));
                            fechar = true;
                        }
                    }
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

    /// O tamanho que vale é o da última resposta (depois dos redirecionamentos).
    #[test]
    fn tamanho_le_o_ultimo_content_length() {
        let f = std::env::temp_dir().join(format!("nider-text-headers-test-{}", std::process::id()));
        std::fs::write(&f, "HTTP/1.1 302 Found\r\nContent-Length: 0\r\n\r\nHTTP/1.1 200 OK\r\ncontent-length: 1234\r\n\r\n").unwrap();
        assert_eq!(tamanho(&f), Some(1234));
        let _ = std::fs::remove_file(&f);
        assert_eq!(tamanho(&f), None);
    }

    #[test]
    fn compara_versoes_ignorando_sufixo() {
        assert!(mais_nova("0.1.6-alfa", "0.1.5-alfa"));
        assert!(mais_nova("v0.2.0", "0.1.9"));
        assert!(!mais_nova("0.1.5-alfa", "0.1.5-alfa"));
        assert!(!mais_nova("0.1.4", "0.1.5"));
        assert!(!mais_nova("lixo", "0.1.5"));
    }
}
