extern crate image as image_crate;

use futures_util::{SinkExt, StreamExt};
use iced::font::{self, Font, Weight};
use iced::widget::image;
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Application, Command, Element, Length, Settings, Subscription, Theme};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as WsMessage};

type WsSender = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    WsMessage,
>;

const CHAT_SCROLL_ID: &str = "chat_scroll_principal";

// Paleta vermelha com roxo escuro como base secundaria. Mantenha cores e
// medidas aqui para evitar valores soltos quando a UI crescer.
const KARU_APP_BG: iced::Color = iced::Color::from_rgb(0.075, 0.043, 0.118); // #130b1e
const KARU_SERVER_BG: iced::Color = iced::Color::from_rgb(0.055, 0.031, 0.094); // #0e0818
const KARU_SIDEBAR: iced::Color = iced::Color::from_rgb(0.102, 0.059, 0.165); // #1a0f2a
const KARU_CHAT_BG: iced::Color = iced::Color::from_rgb(0.075, 0.043, 0.118); // #130b1e
const KARU_PANEL: iced::Color = iced::Color::from_rgb(0.145, 0.086, 0.231); // #25163b
const KARU_INPUT: iced::Color = iced::Color::from_rgb(0.196, 0.118, 0.298); // #321e4c
const KARU_TEXT: iced::Color = iced::Color::from_rgb(0.957, 0.914, 0.929); // #f4e9ed
const KARU_MUTED: iced::Color = iced::Color::from_rgb(0.757, 0.655, 0.737); // #c1a7bc
const KARU_MUTED_DARK: iced::Color = iced::Color::from_rgb(0.494, 0.408, 0.506); // #7e6881
const KARU_ACCENT: iced::Color = iced::Color::from_rgb(0.859, 0.153, 0.247); // #db273f
const KARU_GREEN: iced::Color = iced::Color::from_rgb(0.651, 0.890, 0.631); // #a6e3a1
const KARU_RED: iced::Color = iced::Color::from_rgb(0.859, 0.153, 0.247); // #db273f
const KARU_YELLOW: iced::Color = iced::Color::from_rgb(0.965, 0.757, 0.435); // #f6c16f

const PFP_DIR: &str = "pfps";
const STATUS_WIDTH: f32 = 260.0;
const AVATAR_CACHE_SIZE: u32 = 96;
const LIMITE_HISTORICO_LOCAL: usize = 80;

const FONTE_BOLD: Font = Font {
    family: font::Family::Name("CaskaydiaCove Nerd Font Mono"),
    weight: Weight::Bold,
    ..Font::DEFAULT
};

const FONTE_MONO: Font = Font {
    family: font::Family::Name("CaskaydiaCove Nerd Font Mono"),
    ..Font::DEFAULT
};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
enum ChatProtocol {
    #[serde(rename = "login")]
    Login {
        username: String,
        password: pash::Password,
    },
    #[serde(rename = "register")]
    Register {
        username: String,
        email: String,
        password: pash::Password,
    },
    #[serde(rename = "chat")]
    Chat {
        user: String,
        msg: String,
        #[serde(default)]
        pfp: Option<String>,
        #[serde(default)]
        time: Option<String>,
    },
    #[serde(rename = "join_channel")]
    JoinChannel { channel: String },
    #[serde(rename = "user_list")]
    UserList { users: Vec<UserProfile> },
    #[serde(rename = "pfp_update")]
    PfpUpdate { pfp: String },
    #[serde(rename = "system")]
    System { msg: String },
    #[serde(rename = "auth_response")]
    AuthResponse { success: bool, msg: String },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct UserProfile {
    username: String,
    #[serde(default)]
    pfp: Option<String>,
}

mod pash {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Clone, Debug)]
    pub struct Password(pub String);

    impl Serialize for Password {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(&self.0)
        }
    }

    impl<'de> Deserialize<'de> for Password {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            Ok(Password(s))
        }
    }
}

struct EstiloInputKaru;

impl iced::widget::text_input::StyleSheet for EstiloInputKaru {
    type Style = iced::Theme;

    fn active(&self, _style: &Self::Style) -> iced::widget::text_input::Appearance {
        iced::widget::text_input::Appearance {
            background: iced::Background::Color(KARU_INPUT),
            border: iced::Border {
                color: iced::Color::TRANSPARENT,
                width: 0.0,
                radius: 14.0.into(),
            },
            icon_color: KARU_MUTED,
        }
    }

    fn focused(&self, _style: &Self::Style) -> iced::widget::text_input::Appearance {
        iced::widget::text_input::Appearance {
            background: iced::Background::Color(KARU_INPUT),
            border: iced::Border {
                color: KARU_ACCENT,
                width: 1.0,
                radius: 14.0.into(),
            },
            icon_color: KARU_MUTED,
        }
    }

    fn disabled(&self, style: &Self::Style) -> iced::widget::text_input::Appearance {
        self.active(style)
    }

    fn value_color(&self, _style: &Self::Style) -> iced::Color {
        KARU_TEXT
    }
    fn placeholder_color(&self, _style: &Self::Style) -> iced::Color {
        KARU_MUTED_DARK
    }
    fn disabled_color(&self, _style: &Self::Style) -> iced::Color {
        KARU_MUTED_DARK
    }
    fn selection_color(&self, _style: &Self::Style) -> iced::Color {
        iced::Color {
            r: 0.659,
            g: 0.196,
            b: 0.275,
            a: 0.45,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum SubTelaAuth {
    Login,
    Cadastro,
}

#[derive(Debug, Clone, PartialEq)]
enum TelaAtual {
    Autenticacao(SubTelaAuth),
    Chat,
}

struct Karu {
    tela: TelaAtual,
    username: String,
    input_username: String,
    input_email: String,
    input_password: pash::Password,
    status_erro: String,
    status_conexao: String,
    ws_url: String,
    input_value: String,
    historico_chat: Vec<String>,
    canais: Vec<String>,
    canal_atual: String,
    usuarios_online: Vec<UserProfile>,
    pfps: HashMap<String, String>,
    pfp_handles: HashMap<String, image::Handle>,
    conexao_tx: Option<Arc<Mutex<WsSender>>>,
}

#[derive(Debug, Clone)]
enum Message {
    SubTelaAlterada(SubTelaAuth),
    UsernameAlterado(String),
    EmailAlterado(String),
    PasswordAlterado(String),
    SubmeterAutenticacao,
    InputAlterado(String),
    EnviarMensagem,
    ConexaoPronta(Option<Arc<Mutex<WsSender>>>),
    ConexaoFalhou(String),
    ConexaoEncerrada(String),
    MensagemRecebida(String),
    MudarCanal(String),
}

enum LinhaChat<'a> {
    Sistema(&'a str),
    Mensagem {
        autor: &'a str,
        hora: Option<&'a str>,
        conteudo: &'a str,
    },
}

fn painel(
    cor: iced::Color,
    raio: f32,
) -> impl Fn(&iced::Theme) -> iced::widget::container::Appearance + Clone {
    move |_theme: &iced::Theme| {
        let mut aparencia = iced::widget::container::Appearance::default();
        aparencia.background = Some(iced::Background::Color(cor));
        aparencia.border.radius = if raio > 0.0 {
            (raio + 6.0).into()
        } else {
            raio.into()
        };
        aparencia
    }
}

fn bolha(cor: iced::Color) -> impl Fn(&iced::Theme) -> iced::widget::container::Appearance + Clone {
    move |_theme: &iced::Theme| {
        let mut aparencia = iced::widget::container::Appearance::default();
        aparencia.background = Some(iced::Background::Color(cor));
        aparencia.border.radius = 22.0.into();
        aparencia
    }
}

fn separar_linha_chat(linha: &str) -> LinhaChat<'_> {
    if let Some(sistema) = linha.strip_prefix("[SISTEMA] ") {
        return LinhaChat::Sistema(sistema);
    }

    if let Some(resto) = linha.strip_prefix('[') {
        if let Some((autor, conteudo)) = resto.split_once("]: ") {
            let (autor, hora) = autor
                .split_once('|')
                .map_or((autor, None), |(nome, hora)| (nome, Some(hora)));
            return LinhaChat::Mensagem {
                autor,
                hora,
                conteudo,
            };
        }
    }

    LinhaChat::Sistema(linha)
}

fn inicial_usuario(nome: &str) -> String {
    nome.chars()
        .find(|c| !c.is_whitespace())
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "?".to_string())
}

fn caminho_pfp(usuario: &str) -> Option<PathBuf> {
    let nome_limpo = usuario.trim();
    if nome_limpo.is_empty() {
        return None;
    }

    ["png", "jpg", "jpeg", "webp"]
        .iter()
        .map(|extensao| PathBuf::from(PFP_DIR).join(format!("{}.{}", nome_limpo, extensao)))
        .find(|caminho| caminho.is_file())
}

fn mime_imagem(caminho: &str) -> Option<&'static str> {
    let extensao = PathBuf::from(caminho)
        .extension()?
        .to_string_lossy()
        .to_ascii_lowercase();

    match extensao.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABELA: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut saida = String::with_capacity(bytes.len().div_ceil(3) * 4);

    for bloco in bytes.chunks(3) {
        let b0 = bloco[0];
        let b1 = *bloco.get(1).unwrap_or(&0);
        let b2 = *bloco.get(2).unwrap_or(&0);

        saida.push(TABELA[(b0 >> 2) as usize] as char);
        saida.push(TABELA[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);

        if bloco.len() > 1 {
            saida.push(TABELA[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            saida.push('=');
        }

        if bloco.len() > 2 {
            saida.push(TABELA[(b2 & 0b0011_1111) as usize] as char);
        } else {
            saida.push('=');
        }
    }

    saida
}

fn valor_base64(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

fn base64_decode(texto: &str) -> Option<Vec<u8>> {
    let bytes: Vec<u8> = texto.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if bytes.len() % 4 != 0 {
        return None;
    }

    let mut saida = Vec::with_capacity(bytes.len() / 4 * 3);
    for bloco in bytes.chunks(4) {
        let v0 = valor_base64(bloco[0])?;
        let v1 = valor_base64(bloco[1])?;
        let v2 = if bloco[2] == b'=' {
            0
        } else {
            valor_base64(bloco[2])?
        };
        let v3 = if bloco[3] == b'=' {
            0
        } else {
            valor_base64(bloco[3])?
        };

        saida.push((v0 << 2) | (v1 >> 4));
        if bloco[2] != b'=' {
            saida.push(((v1 & 0b0000_1111) << 4) | (v2 >> 2));
        }
        if bloco[3] != b'=' {
            saida.push(((v2 & 0b0000_0011) << 6) | v3);
        }
    }

    Some(saida)
}

fn decodificar_data_url_imagem(data_url: &str) -> Option<Vec<u8>> {
    let (cabecalho, dados) = data_url.split_once(',')?;
    if !cabecalho.starts_with("data:image/") || !cabecalho.ends_with(";base64") {
        return None;
    }

    base64_decode(dados)
}

fn imagem_circular(bytes: &[u8], tamanho: u32) -> Option<Vec<u8>> {
    let imagem = image_crate::load_from_memory(bytes).ok()?;
    let mut avatar = imagem
        .resize_to_fill(
            tamanho,
            tamanho,
            image_crate::imageops::FilterType::Lanczos3,
        )
        .to_rgba8();
    let raio = tamanho as f32 / 2.0;
    let centro = raio - 0.5;

    for (x, y, pixel) in avatar.enumerate_pixels_mut() {
        let dx = x as f32 - centro;
        let dy = y as f32 - centro;
        if (dx * dx + dy * dy).sqrt() > raio {
            pixel[3] = 0;
        }
    }

    let mut png = Vec::new();
    image_crate::DynamicImage::ImageRgba8(avatar)
        .write_to(
            &mut Cursor::new(&mut png),
            image_crate::ImageOutputFormat::Png,
        )
        .ok()?;
    Some(png)
}

fn avatar_handle_de_data_url(data_url: &str) -> Option<image::Handle> {
    decodificar_data_url_imagem(data_url)
        .and_then(|bytes| imagem_circular(&bytes, AVATAR_CACHE_SIZE))
        .map(image::Handle::from_memory)
}

fn imagem_para_data_url(caminho: &str) -> Result<String, String> {
    let mime = mime_imagem(caminho)
        .ok_or_else(|| "Use uma imagem .png, .jpg, .jpeg ou .webp para a PFP.".to_string())?;
    let bytes = fs::read(caminho).map_err(|erro| format!("Não consegui ler a imagem: {}", erro))?;

    if bytes.len() > 1_000_000 {
        return Err("A imagem é grande demais. Use uma PFP com até 1 MB.".to_string());
    }

    Ok(format!("data:{};base64,{}", mime, base64_encode(&bytes)))
}

fn limpar_markdown_inline(texto: &str) -> String {
    texto
        .replace("**", "")
        .replace("__", "")
        .replace('*', "")
        .replace('`', "")
}

fn detectar_cerca_codigo(linha: &str) -> Option<(&'static str, &str)> {
    let aparada = linha.trim_start();

    if let Some(resto) = aparada.strip_prefix("```") {
        return Some(("```", resto));
    }

    if let Some(resto) = aparada.strip_prefix("'''") {
        return Some(("'''", resto));
    }

    None
}

impl Karu {
    fn view_avatar<'a>(
        &'a self,
        usuario: &'a str,
        tamanho: f32,
        cor_fallback: iced::Color,
    ) -> Element<'a, Message> {
        if let Some(handle) = self.pfp_handles.get(usuario) {
            return container(
                image(handle.clone())
                    .width(Length::Fixed(tamanho))
                    .height(Length::Fixed(tamanho))
                    .content_fit(iced::ContentFit::Cover),
            )
            .width(Length::Fixed(tamanho))
            .height(Length::Fixed(tamanho))
            .style(painel(KARU_SERVER_BG, tamanho / 2.0))
            .into();
        }

        if let Some(bytes) = caminho_pfp(usuario)
            .and_then(|caminho| fs::read(caminho).ok())
            .and_then(|bytes| imagem_circular(&bytes, tamanho.ceil() as u32))
        {
            return container(
                image(image::Handle::from_memory(bytes))
                    .width(Length::Fixed(tamanho))
                    .height(Length::Fixed(tamanho))
                    .content_fit(iced::ContentFit::Cover),
            )
            .width(Length::Fixed(tamanho))
            .height(Length::Fixed(tamanho))
            .style(painel(KARU_SERVER_BG, tamanho / 2.0))
            .into();
        }

        container(text(inicial_usuario(usuario)).font(FONTE_BOLD))
            .width(Length::Fixed(tamanho))
            .height(Length::Fixed(tamanho))
            .center_x()
            .center_y()
            .style(bolha(cor_fallback))
            .into()
    }

    fn view_auth(&self, aba: &SubTelaAuth) -> Element<'_, Message> {
        let titulo = text("Karu")
            .size(48)
            .font(FONTE_BOLD)
            .style(iced::theme::Text::Color(KARU_TEXT));

        let subtitulo = text("Entre para conversar nos canais")
            .size(16)
            .font(FONTE_MONO)
            .style(iced::theme::Text::Color(KARU_MUTED));

        let status_conexao = text(format!("{} :: {}", self.status_conexao, self.ws_url))
            .size(12)
            .font(FONTE_MONO)
            .style(iced::theme::Text::Color(if self.conexao_tx.is_some() {
                KARU_GREEN
            } else {
                KARU_MUTED_DARK
            }));

        let btn_login_estilo = if *aba == SubTelaAuth::Login {
            iced::theme::Button::Primary
        } else {
            iced::theme::Button::Secondary
        };
        let btn_cadastro_estilo = if *aba == SubTelaAuth::Cadastro {
            iced::theme::Button::Primary
        } else {
            iced::theme::Button::Secondary
        };

        let seletor_abas = row![
            button(text("Entrar").font(FONTE_BOLD))
                .padding(10)
                .style(btn_login_estilo)
                .on_press(Message::SubTelaAlterada(SubTelaAuth::Login)),
            button(text("Criar conta").font(FONTE_BOLD))
                .padding(10)
                .style(btn_cadastro_estilo)
                .on_press(Message::SubTelaAlterada(SubTelaAuth::Cadastro))
        ]
        .spacing(10);

        let mut formulario = column![].spacing(12).align_items(Alignment::Center);

        formulario = formulario.push(
            text_input("Nome de usuário", &self.input_username)
                .on_input(Message::UsernameAlterado)
                .font(FONTE_MONO)
                .style(iced::theme::TextInput::Custom(Box::new(EstiloInputKaru)))
                .padding(13)
                .width(Length::Fixed(340.0)),
        );

        if *aba == SubTelaAuth::Cadastro {
            formulario = formulario.push(
                text_input("E-mail", &self.input_email)
                    .on_input(Message::EmailAlterado)
                    .font(FONTE_MONO)
                    .style(iced::theme::TextInput::Custom(Box::new(EstiloInputKaru)))
                    .padding(13)
                    .width(Length::Fixed(340.0)),
            );
        }

        formulario = formulario.push(
            text_input("Senha", &self.input_password.0)
                .on_input(Message::PasswordAlterado)
                .on_submit(Message::SubmeterAutenticacao)
                .secure(true)
                .font(FONTE_MONO)
                .style(iced::theme::TextInput::Custom(Box::new(EstiloInputKaru)))
                .padding(13)
                .width(Length::Fixed(340.0)),
        );

        let label_acao = if *aba == SubTelaAuth::Login {
            "Entrar"
        } else {
            "Criar conta"
        };
        let botao_confirmar = button(text(label_acao).font(FONTE_BOLD))
            .padding(12)
            .width(Length::Fixed(180.0))
            .on_press(Message::SubmeterAutenticacao)
            .style(iced::theme::Button::Primary);

        let status =
            text(&self.status_erro)
                .size(14)
                .font(FONTE_MONO)
                .style(iced::theme::Text::Color(
                    if self.status_erro.contains("Conta criada") {
                        KARU_GREEN
                    } else {
                        KARU_RED
                    },
                ));

        let card = container(
            column![
                titulo,
                subtitulo,
                status_conexao,
                seletor_abas,
                formulario,
                botao_confirmar,
                status
            ]
            .spacing(18)
            .align_items(Alignment::Center),
        )
        .padding(28)
        .style(painel(KARU_PANEL, 10.0));

        container(card)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .style(painel(KARU_APP_BG, 0.0))
            .into()
    }

    fn view_channel_topbar(&self) -> Element<'_, Message> {
        let mut abas = row![text("KARU")
            .font(FONTE_BOLD)
            .size(18)
            .style(iced::theme::Text::Color(KARU_TEXT))]
        .spacing(10)
        .align_items(Alignment::Center);

        for canal in &self.canais {
            let ativo = *canal == self.canal_atual;
            let estilo = if ativo {
                iced::theme::Button::Primary
            } else {
                iced::theme::Button::Secondary
            };

            abas = abas.push(
                button(
                    row![
                        text("::")
                            .font(FONTE_BOLD)
                            .style(iced::theme::Text::Color(KARU_MUTED)),
                        text(canal).font(FONTE_BOLD)
                    ]
                    .spacing(8)
                    .align_items(Alignment::Center),
                )
                .padding([8, 12])
                .style(estilo)
                .on_press(Message::MudarCanal(canal.clone())),
            );
        }

        let perfil = row![
            text(format!("[{}]", self.canal_atual))
                .font(FONTE_BOLD)
                .size(14)
                .style(iced::theme::Text::Color(KARU_ACCENT)),
            row![
                self.view_avatar(&self.username, 32.0, KARU_ACCENT),
                column![
                    text(if self.username.is_empty() {
                        "desconectado"
                    } else {
                        &self.username
                    })
                    .font(FONTE_BOLD)
                    .size(14),
                    text(if self.conexao_tx.is_some() {
                        "online"
                    } else {
                        "offline"
                    })
                    .font(FONTE_MONO)
                    .size(12)
                    .style(iced::theme::Text::Color(
                        if self.conexao_tx.is_some() {
                            KARU_GREEN
                        } else {
                            KARU_MUTED_DARK
                        }
                    ))
                ]
                .spacing(2)
            ]
            .spacing(10)
            .align_items(Alignment::Center)
        ]
        .spacing(16)
        .align_items(Alignment::Center);

        container(
            row![
                container(abas).width(Length::Fill),
                container(perfil).width(Length::Shrink)
            ]
            .spacing(16)
            .align_items(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fixed(58.0))
        .padding([10, 14])
        .style(painel(KARU_SERVER_BG, 0.0))
        .into()
    }

    fn view_system_message<'a>(&'a self, msg: &'a str) -> Element<'a, Message> {
        container(
            row![
                text("!")
                    .font(FONTE_BOLD)
                    .style(iced::theme::Text::Color(KARU_YELLOW)),
                text(msg)
                    .size(14)
                    .font(FONTE_MONO)
                    .style(iced::theme::Text::Color(KARU_MUTED))
            ]
            .spacing(10)
            .align_items(Alignment::Center),
        )
        .padding([6, 10])
        .style(painel(KARU_PANEL, 6.0))
        .into()
    }

    fn view_message_group<'a>(
        &'a self,
        autor: &'a str,
        hora: Option<&'a str>,
        mensagens: &[&'a str],
    ) -> Element<'a, Message> {
        let autor_eh_usuario = autor == self.username;
        let cor_avatar = if autor_eh_usuario {
            KARU_ACCENT
        } else {
            KARU_PANEL
        };

        let mut corpo = column![row![
            text(autor)
                .font(FONTE_BOLD)
                .size(15)
                .style(iced::theme::Text::Color(if autor_eh_usuario {
                    KARU_ACCENT
                } else {
                    KARU_TEXT
                })),
            text(hora.unwrap_or("agora"))
                .size(12)
                .font(FONTE_MONO)
                .style(iced::theme::Text::Color(KARU_MUTED_DARK))
        ]
        .spacing(8)
        .align_items(Alignment::Center)]
        .spacing(2)
        .width(Length::Fill);

        for conteudo in mensagens {
            corpo = corpo.push(self.view_markdown(conteudo));
        }

        row![self.view_avatar(autor, 38.0, cor_avatar), corpo]
            .spacing(12)
            .padding([8, 8])
            .align_items(Alignment::Start)
            .into()
    }

    fn view_code_block<'a>(&self, codigo: String) -> Element<'a, Message> {
        container(
            text(codigo)
                .font(FONTE_MONO)
                .size(14)
                .style(iced::theme::Text::Color(KARU_TEXT)),
        )
        .width(Length::Fill)
        .padding(10)
        .style(painel(KARU_SERVER_BG, 6.0))
        .into()
    }

    fn view_markdown_line<'a>(&self, linha: &str) -> Element<'a, Message> {
        let aparada = linha.trim();

        if let Some(titulo) = aparada.strip_prefix("### ") {
            return text(limpar_markdown_inline(titulo))
                .font(FONTE_BOLD)
                .size(16)
                .style(iced::theme::Text::Color(KARU_TEXT))
                .into();
        }

        if let Some(titulo) = aparada.strip_prefix("## ") {
            return text(limpar_markdown_inline(titulo))
                .font(FONTE_BOLD)
                .size(18)
                .style(iced::theme::Text::Color(KARU_TEXT))
                .into();
        }

        if let Some(titulo) = aparada.strip_prefix("# ") {
            return text(limpar_markdown_inline(titulo))
                .font(FONTE_BOLD)
                .size(20)
                .style(iced::theme::Text::Color(KARU_ACCENT))
                .into();
        }

        if let Some(citado) = aparada.strip_prefix("> ") {
            return container(
                row![
                    text("|")
                        .font(FONTE_BOLD)
                        .style(iced::theme::Text::Color(KARU_ACCENT)),
                    text(limpar_markdown_inline(citado))
                        .font(FONTE_MONO)
                        .size(15)
                        .style(iced::theme::Text::Color(KARU_MUTED))
                ]
                .spacing(8),
            )
            .padding([2, 0])
            .into();
        }

        if let Some(item) = aparada
            .strip_prefix("- ")
            .or_else(|| aparada.strip_prefix("* "))
        {
            return row![
                text("-")
                    .font(FONTE_BOLD)
                    .style(iced::theme::Text::Color(KARU_ACCENT)),
                text(limpar_markdown_inline(item))
                    .font(FONTE_MONO)
                    .size(15)
                    .style(iced::theme::Text::Color(KARU_TEXT))
            ]
            .spacing(8)
            .into();
        }

        if let Some((numero, item)) = aparada.split_once(". ") {
            if numero.chars().all(|c| c.is_ascii_digit()) {
                return row![
                    text(format!("{}.", numero))
                        .font(FONTE_BOLD)
                        .style(iced::theme::Text::Color(KARU_ACCENT)),
                    text(limpar_markdown_inline(item))
                        .font(FONTE_MONO)
                        .size(15)
                        .style(iced::theme::Text::Color(KARU_TEXT))
                ]
                .spacing(8)
                .into();
            }
        }

        text(limpar_markdown_inline(aparada))
            .font(FONTE_MONO)
            .size(15)
            .style(iced::theme::Text::Color(KARU_TEXT))
            .into()
    }

    fn view_markdown<'a>(&'a self, conteudo: &'a str) -> Element<'a, Message> {
        let mut blocos = column!().spacing(4).width(Length::Fill);
        let mut em_codigo = false;
        let mut codigo = String::new();
        let mut cerca_atual = "";
        let mut teve_linha = false;

        for linha in conteudo.lines() {
            teve_linha = true;

            if let Some((cerca, resto)) = detectar_cerca_codigo(linha) {
                if em_codigo && cerca == cerca_atual {
                    blocos = blocos.push(self.view_code_block(std::mem::take(&mut codigo)));
                    em_codigo = false;
                    cerca_atual = "";
                    continue;
                }

                if !em_codigo {
                    if let Some((codigo_inline, _)) = resto.split_once(cerca) {
                        blocos =
                            blocos.push(self.view_code_block(codigo_inline.trim().to_string()));
                    } else {
                        codigo.clear();
                        let primeira_linha = resto.trim();
                        if !primeira_linha.is_empty() && cerca == "'''" {
                            codigo.push_str(primeira_linha);
                        }
                        cerca_atual = cerca;
                        em_codigo = true;
                    }
                    continue;
                }
            }

            if em_codigo {
                if !codigo.is_empty() {
                    codigo.push('\n');
                }
                codigo.push_str(linha);
            } else if linha.trim().is_empty() {
                blocos = blocos.push(text("").size(6));
            } else {
                blocos = blocos.push(self.view_markdown_line(linha));
            }
        }

        if em_codigo {
            blocos = blocos.push(self.view_code_block(codigo));
        }

        if !teve_linha {
            blocos = blocos.push(self.view_markdown_line(conteudo));
        }

        container(blocos).width(Length::Fill).into()
    }

    fn view_chat_center(&self) -> Element<'_, Message> {
        let header = container(
            row![
                text("::")
                    .font(FONTE_BOLD)
                    .size(22)
                    .style(iced::theme::Text::Color(KARU_MUTED)),
                text(&self.canal_atual).font(FONTE_BOLD).size(18),
                text(format!("{} mensagens", self.historico_chat.len()))
                    .size(13)
                    .font(FONTE_MONO)
                    .style(iced::theme::Text::Color(KARU_MUTED_DARK))
            ]
            .spacing(10)
            .align_items(Alignment::Center),
        )
        .height(Length::Fixed(46.0))
        .padding([0, 18])
        .style(painel(KARU_CHAT_BG, 0.0));

        let mut chat_content = column!().spacing(10).padding([14, 18]);
        let mut indice = 0;

        while indice < self.historico_chat.len() {
            match separar_linha_chat(&self.historico_chat[indice]) {
                LinhaChat::Sistema(msg) => {
                    chat_content = chat_content.push(self.view_system_message(msg));
                    indice += 1;
                }
                LinhaChat::Mensagem {
                    autor,
                    hora,
                    conteudo,
                } => {
                    let mut mensagens = vec![conteudo];
                    let mut proximo = indice + 1;

                    while proximo < self.historico_chat.len() {
                        match separar_linha_chat(&self.historico_chat[proximo]) {
                            LinhaChat::Mensagem {
                                autor: proximo_autor,
                                conteudo: proximo_conteudo,
                                ..
                            } if proximo_autor == autor => {
                                mensagens.push(proximo_conteudo);
                                proximo += 1;
                            }
                            _ => break,
                        }
                    }

                    chat_content =
                        chat_content.push(self.view_message_group(autor, hora, &mensagens));
                    indice = proximo;
                }
            }
        }

        let area_mensagens = scrollable(chat_content)
            .id(scrollable::Id::new(CHAT_SCROLL_ID))
            .height(Length::Fill)
            .width(Length::Fill);

        let composer = container(
            text_input("Conversar neste canal", &self.input_value)
                .on_input(Message::InputAlterado)
                .on_submit(Message::EnviarMensagem)
                .font(FONTE_MONO)
                .size(16)
                .padding(14)
                .style(iced::theme::TextInput::Custom(Box::new(EstiloInputKaru))),
        )
        .padding([0, 18, 18, 18]);

        container(column![header, area_mensagens, composer])
            .width(Length::Fill)
            .height(Length::Fill)
            .style(painel(KARU_CHAT_BG, 0.0))
            .into()
    }

    fn view_status_panel(&self) -> Element<'_, Message> {
        let conexao = if self.conexao_tx.is_some() {
            "ws conectado"
        } else {
            "ws offline"
        };

        let mut usuarios = column![].spacing(6);
        for perfil in &self.usuarios_online {
            usuarios = usuarios.push(
                row![
                    self.view_avatar(&perfil.username, 24.0, KARU_PANEL),
                    text(&perfil.username).font(FONTE_MONO).size(13).style(
                        iced::theme::Text::Color(if perfil.username == self.username {
                            KARU_GREEN
                        } else {
                            KARU_MUTED
                        })
                    )
                ]
                .spacing(8)
                .align_items(Alignment::Center),
            );
        }

        let painel_status = column![
            text("STATUS")
                .font(FONTE_BOLD)
                .size(13)
                .style(iced::theme::Text::Color(KARU_ACCENT)),
            text(conexao)
                .font(FONTE_MONO)
                .size(13)
                .style(iced::theme::Text::Color(if self.conexao_tx.is_some() {
                    KARU_GREEN
                } else {
                    KARU_RED
                })),
            text(format!("canal :: {}", self.canal_atual))
                .font(FONTE_MONO)
                .size(13)
                .style(iced::theme::Text::Color(KARU_MUTED)),
            text(format!("online :: {}", self.usuarios_online.len()))
                .font(FONTE_MONO)
                .size(13)
                .style(iced::theme::Text::Color(KARU_MUTED)),
            text("ATALHOS")
                .font(FONTE_BOLD)
                .size(13)
                .style(iced::theme::Text::Color(KARU_ACCENT)),
            text("enter :: enviar")
                .font(FONTE_MONO)
                .size(13)
                .style(iced::theme::Text::Color(KARU_MUTED_DARK)),
            text("``` :: bloco codigo")
                .font(FONTE_MONO)
                .size(13)
                .style(iced::theme::Text::Color(KARU_MUTED_DARK)),
            text("/pfp caminho :: foto")
                .font(FONTE_MONO)
                .size(13)
                .style(iced::theme::Text::Color(KARU_MUTED_DARK)),
            text("# :: titulo")
                .font(FONTE_MONO)
                .size(13)
                .style(iced::theme::Text::Color(KARU_MUTED_DARK)),
            text("PRESENCAS")
                .font(FONTE_BOLD)
                .size(13)
                .style(iced::theme::Text::Color(KARU_ACCENT)),
            usuarios
        ]
        .spacing(10);

        container(painel_status)
            .width(Length::Fixed(STATUS_WIDTH))
            .height(Length::Fill)
            .padding(16)
            .style(painel(KARU_SIDEBAR, 0.0))
            .into()
    }

    fn view_chat(&self) -> Element<'_, Message> {
        if std::env::var("KARU_UI_LITE").is_ok() {
            return column![self.view_channel_topbar(), self.view_chat_center()]
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }

        column![
            self.view_channel_topbar(),
            row![self.view_chat_center(), self.view_status_panel()]
                .width(Length::Fill)
                .height(Length::Fill)
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

impl Application for Karu {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Karu {
                tela: TelaAtual::Autenticacao(SubTelaAuth::Login),
                username: String::new(),
                input_username: String::new(),
                input_email: String::new(),
                input_password: pash::Password(String::new()),
                status_erro: String::new(),
                status_conexao: "conectando ws".to_string(),
                ws_url: std::env::var("KARU_WS_URL")
                    .unwrap_or_else(|_| "ws://localhost:8765".to_string()),
                input_value: String::new(),
                historico_chat: Vec::new(),
                canais: vec!["geral".to_string(), "dev".to_string(), "ajuda".to_string()],
                canal_atual: "geral".to_string(),
                usuarios_online: Vec::new(),
                pfps: HashMap::new(),
                pfp_handles: HashMap::new(),
                conexao_tx: None,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        match self.tela {
            TelaAtual::Autenticacao(_) => "Karu - Autenticação Segura".to_string(),
            TelaAtual::Chat => format!("Karu - [#{}] como {}", self.canal_atual, self.username),
        }
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::SubTelaAlterada(aba) => {
                self.tela = TelaAtual::Autenticacao(aba);
                self.status_erro.clear();
            }
            Message::UsernameAlterado(val) => {
                self.input_username = val;
            }
            Message::EmailAlterado(val) => {
                self.input_email = val;
            }
            Message::PasswordAlterado(val) => {
                self.input_password = pash::Password(val);
            }

            Message::SubmeterAutenticacao => {
                if self.input_username.is_empty() || self.input_password.0.is_empty() {
                    self.status_erro = "Preencha os campos obrigatórios!".to_string();
                    return Command::none();
                }

                if let Some(tx) = &self.conexao_tx {
                    let tx_clone = Arc::clone(tx);

                    let payload = match &self.tela {
                        TelaAtual::Autenticacao(SubTelaAuth::Login) => {
                            serde_json::to_string(&ChatProtocol::Login {
                                username: self.input_username.clone(),
                                password: self.input_password.clone(),
                            })
                            .unwrap()
                        }
                        TelaAtual::Autenticacao(SubTelaAuth::Cadastro) => {
                            if self.input_email.is_empty() {
                                self.status_erro = "E-mail é obrigatório!".to_string();
                                return Command::none();
                            }
                            serde_json::to_string(&ChatProtocol::Register {
                                username: self.input_username.clone(),
                                email: self.input_email.clone(),
                                password: self.input_password.clone(),
                            })
                            .unwrap()
                        }
                        _ => return Command::none(),
                    };

                    return Command::perform(
                        async move {
                            let mut sender = tx_clone.lock().await;
                            sender
                                .send(WsMessage::Text(payload))
                                .await
                                .map_err(|erro| erro.to_string())
                        },
                        |resultado| match resultado {
                            Ok(()) => Message::InputAlterado("".to_string()),
                            Err(erro) => {
                                Message::ConexaoFalhou(format!("Falha ao enviar auth: {}", erro))
                            }
                        },
                    );
                }

                self.status_erro = format!("Sem conexão WebSocket em {}", self.ws_url);
            }

            Message::InputAlterado(val) => {
                self.input_value = val;
            }

            Message::EnviarMensagem => {
                if !self.input_value.is_empty() {
                    if let Some(caminho) = self.input_value.trim().strip_prefix("/pfp ") {
                        match imagem_para_data_url(caminho.trim()) {
                            Ok(pfp) => {
                                if let Some(tx) = &self.conexao_tx {
                                    let payload = serde_json::to_string(&ChatProtocol::PfpUpdate {
                                        pfp: pfp.clone(),
                                    })
                                    .unwrap();
                                    let tx_clone = Arc::clone(tx);

                                    if let Some(handle) = avatar_handle_de_data_url(&pfp) {
                                        self.pfp_handles.insert(self.username.clone(), handle);
                                    }
                                    self.pfps.insert(self.username.clone(), pfp);
                                    self.historico_chat.push(
                                        "[SISTEMA] PFP atualizada e enviada ao servidor."
                                            .to_string(),
                                    );
                                    self.input_value.clear();

                                    return Command::perform(
                                        async move {
                                            let mut sender = tx_clone.lock().await;
                                            let _ = sender.send(WsMessage::Text(payload)).await;
                                        },
                                        |_| Message::InputAlterado("".to_string()),
                                    );
                                }
                            }
                            Err(erro) => {
                                self.historico_chat.push(format!("[SISTEMA] {}", erro));
                                self.input_value.clear();
                            }
                        }

                        return Command::none();
                    }

                    if let Some(tx) = &self.conexao_tx {
                        let payload = serde_json::to_string(&ChatProtocol::Chat {
                            user: self.username.clone(),
                            msg: self.input_value.clone(),
                            pfp: None,
                            time: None,
                        })
                        .unwrap();

                        let tx_clone = Arc::clone(tx);
                        self.input_value.clear();

                        return Command::batch(vec![
                            Command::perform(
                                async move {
                                    let mut sender = tx_clone.lock().await;
                                    let _ = sender.send(WsMessage::Text(payload)).await;
                                },
                                |_| Message::InputAlterado("".to_string()),
                            ),
                            scrollable::scroll_to(
                                scrollable::Id::new(CHAT_SCROLL_ID),
                                scrollable::AbsoluteOffset {
                                    y: f32::MAX,
                                    x: 0.0,
                                },
                            ),
                        ]);
                    }
                }
            }

            Message::ConexaoPronta(tx) => {
                self.status_conexao = if tx.is_some() {
                    "ws conectado".to_string()
                } else {
                    "ws offline".to_string()
                };
                self.conexao_tx = tx;
            }

            Message::ConexaoFalhou(erro) => {
                self.conexao_tx = None;
                self.status_conexao = "ws falhou".to_string();
                self.status_erro = erro;
            }

            Message::ConexaoEncerrada(motivo) => {
                self.conexao_tx = None;
                self.status_conexao = "ws desconectado".to_string();
                if matches!(self.tela, TelaAtual::Autenticacao(_)) {
                    self.status_erro = motivo;
                } else {
                    self.historico_chat.push(format!("[SISTEMA] {}", motivo));
                }
            }

            Message::MensagemRecebida(raw_json) => {
                if let Ok(protocol) = serde_json::from_str::<ChatProtocol>(&raw_json) {
                    let mut precisa_scrollar = false;

                    match protocol {
                        ChatProtocol::AuthResponse { success, msg } => {
                            if success {
                                if let TelaAtual::Autenticacao(SubTelaAuth::Login) = self.tela {
                                    self.username = self.input_username.clone();
                                    self.tela = TelaAtual::Chat;
                                    self.historico_chat.clear();
                                    self.status_erro.clear();
                                } else {
                                    self.tela = TelaAtual::Autenticacao(SubTelaAuth::Login);
                                    self.status_erro = "Conta criada! Faça login.".to_string();
                                }
                            } else {
                                self.status_erro = msg;
                            }
                        }
                        ChatProtocol::Chat {
                            user,
                            msg,
                            pfp,
                            time,
                        } => {
                            if let Some(pfp) = pfp {
                                if self.pfps.get(&user) != Some(&pfp) {
                                    if let Some(handle) = avatar_handle_de_data_url(&pfp) {
                                        self.pfp_handles.insert(user.clone(), handle);
                                    }
                                    self.pfps.insert(user.clone(), pfp);
                                }
                            }
                            let formatado = if let Some(time) = time {
                                format!("[{}|{}]: {}", user, time, msg)
                            } else {
                                format!("[{}]: {}", user, msg)
                            };
                            self.historico_chat.push(formatado);
                            precisa_scrollar = true;
                        }
                        ChatProtocol::UserList { users } => {
                            for perfil in &users {
                                if let Some(pfp) = &perfil.pfp {
                                    if self.pfps.get(&perfil.username) != Some(pfp) {
                                        if let Some(handle) = avatar_handle_de_data_url(pfp) {
                                            self.pfp_handles
                                                .insert(perfil.username.clone(), handle);
                                        }
                                        self.pfps.insert(perfil.username.clone(), pfp.clone());
                                    }
                                }
                            }
                            self.usuarios_online = users;
                        }
                        ChatProtocol::System { msg } => {
                            self.historico_chat.push(format!("[SISTEMA] {}", msg));
                            precisa_scrollar = true;
                        }
                        _ => {}
                    }

                    if self.historico_chat.len() > LIMITE_HISTORICO_LOCAL {
                        let excedente = self.historico_chat.len() - LIMITE_HISTORICO_LOCAL;
                        self.historico_chat.drain(0..excedente);
                    }

                    if precisa_scrollar {
                        return scrollable::scroll_to(
                            scrollable::Id::new(CHAT_SCROLL_ID),
                            scrollable::AbsoluteOffset {
                                y: f32::MAX,
                                x: 0.0,
                            },
                        );
                    }
                }
            }

            Message::MudarCanal(nome_canal) => {
                self.canal_atual = nome_canal.clone();
                self.historico_chat.clear();

                if let Some(tx) = &self.conexao_tx {
                    let tx_clone = Arc::clone(tx);
                    let payload = serde_json::to_string(&ChatProtocol::JoinChannel {
                        channel: nome_canal,
                    })
                    .unwrap();
                    return Command::perform(
                        async move {
                            let mut sender = tx_clone.lock().await;
                            let _ = sender.send(WsMessage::Text(payload)).await;
                        },
                        |_| Message::InputAlterado("".to_string()),
                    );
                }
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        match &self.tela {
            TelaAtual::Autenticacao(aba) => self.view_auth(aba),
            TelaAtual::Chat => self.view_chat(),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        struct NetworkConnection;
        iced::subscription::channel(
            std::any::TypeId::of::<NetworkConnection>(),
            100,
            |mut output| async move {
                let url = std::env::var("KARU_WS_URL")
                    .unwrap_or_else(|_| "ws://localhost:8765".to_string());

                match connect_async(&url).await {
                    Ok((stream, _)) => {
                        let (tx, mut rx) = stream.split();
                        let tx_protegido = Arc::new(Mutex::new(tx));
                        let _ = output
                            .send(Message::ConexaoPronta(Some(tx_protegido)))
                            .await;

                        while let Some(resultado) = rx.next().await {
                            match resultado {
                                Ok(WsMessage::Text(txt)) => {
                                    let _ = output.send(Message::MensagemRecebida(txt)).await;
                                }
                                Ok(WsMessage::Close(frame)) => {
                                    let motivo = frame
                                        .map(|f| f.reason.to_string())
                                        .filter(|reason| !reason.is_empty())
                                        .unwrap_or_else(|| {
                                            "servidor fechou o websocket".to_string()
                                        });
                                    let _ = output.send(Message::ConexaoEncerrada(motivo)).await;
                                    break;
                                }
                                Ok(_) => {}
                                Err(erro) => {
                                    let _ = output
                                        .send(Message::ConexaoEncerrada(format!(
                                            "WebSocket caiu: {}",
                                            erro
                                        )))
                                        .await;
                                    break;
                                }
                            }
                        }
                    }
                    Err(erro) => {
                        let _ = output
                            .send(Message::ConexaoFalhou(format!(
                                "Não conectou em {}: {}",
                                url, erro
                            )))
                            .await;
                    }
                }
                let _ = output.send(Message::ConexaoPronta(None)).await;
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
                }
            },
        )
    }

    fn theme(&self) -> Theme {
        Theme::custom(
            "CatppuccinMochaKaru".to_string(),
            iced::theme::Palette {
                background: KARU_APP_BG,
                text: KARU_TEXT,
                primary: KARU_ACCENT,
                success: KARU_GREEN,
                danger: KARU_RED,
            },
        )
    }
}

fn main() -> iced::Result {
    Karu::run(Settings {
        window: iced::window::Settings {
            size: iced::Size::new(1180.0, 720.0),
            ..Default::default()
        },
        ..Default::default()
    })
}
