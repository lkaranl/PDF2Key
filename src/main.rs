//! PDF2Key - Conversor de PDF para Apple Keynote
//! 
//! Aplicação desktop que converte arquivos PDF em apresentações .key editáveis

mod pdf_processor;
mod keynote;

use anyhow::Result;
use eframe::egui;
use image::ImageFormat;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::process::Command;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([550.0, 450.0])
            .with_min_inner_size([500.0, 400.0])
            .with_title("PDF2Key")
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "PDF2Key",
        options,
        Box::new(|cc| {
            // Configura estilo visual clean
            let mut style = (*cc.egui_ctx.style()).clone();
            
            // Espaçamento e Layout
            style.spacing.item_spacing = egui::vec2(16.0, 16.0);
            style.spacing.button_padding = egui::vec2(24.0, 12.0);
            style.visuals.widgets.noninteractive.bg_stroke.width = 0.0;
            
            // Fontes mais modernas e legíveis
            style.text_styles.insert(
                egui::TextStyle::Heading,
                egui::FontId::new(26.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(15.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Button,
                egui::FontId::new(16.0, egui::FontFamily::Proportional),
            );
            
            cc.egui_ctx.set_style(style);
            
            Ok(Box::new(Pdf2KeyApp::default()))
        }),
    )
}

#[derive(Default)]
struct Pdf2KeyApp {
    pdf_path: Option<PathBuf>,
    output_path: Option<PathBuf>,
    status: Arc<Mutex<AppStatus>>,
    is_converting: Arc<Mutex<bool>>,
}

#[derive(Default, Clone)]
struct AppStatus {
    message: String,
    progress: f32,
    is_error: bool,
    is_success: bool,
}

impl eframe::App for Pdf2KeyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if *self.is_converting.lock().unwrap() {
            ctx.request_repaint();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // Centraliza verticalmente e horizontalmente com limite de largura
            ui.vertical_centered(|ui| {
                ui.set_max_width(450.0); // Limita largura para manter design coeso
                ui.add_space(20.0);
                
                // Cabeçalho
                ui.heading(egui::RichText::new("📄 PDF2Key").strong());
                ui.label(egui::RichText::new("Suas apresentações, prontas em segundos.").color(egui::Color32::GRAY));
                
                ui.add_space(30.0);
                
                // Área de Upload (Card Grande)
                let card_response = egui::Frame::group(ui.style())
                    .inner_margin(30.0)
                    .rounding(12.0)
                    .stroke(egui::Stroke::new(1.5, egui::Color32::from_gray(200))) // Borda suave
                    .fill(if ui.visuals().dark_mode { egui::Color32::from_gray(30) } else { egui::Color32::from_gray(250) })
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.vertical_centered(|ui| {
                            if let Some(path) = &self.pdf_path {
                                ui.label(egui::RichText::new("✅ Arquivo Selecionado").color(egui::Color32::from_rgb(50, 180, 50)).strong());
                                ui.add_space(8.0);
                                ui.label(egui::RichText::new(path.file_name().unwrap_or_default().to_string_lossy())
                                    .size(16.0)
                                    .strong());
                                ui.add_space(8.0);
                                if ui.link("Escolher outro arquivo").clicked() {
                                    self.select_pdf();
                                }
                            } else {
                                ui.label(egui::RichText::new("📂").size(32.0));
                                ui.add_space(8.0);
                                ui.label(egui::RichText::new("Clique para selecionar").size(16.0).strong());
                                ui.label(egui::RichText::new("Selecione um arquivo PDF").size(13.0).color(egui::Color32::GRAY));
                            }
                        });
                    }).response;

                // Clique no card inteiro ativa seleção
                if self.pdf_path.is_none() && card_response.interact(egui::Sense::click()).clicked() {
                     self.select_pdf();
                }

                ui.add_space(30.0);
                
                // Ação Principal
                let is_converting = *self.is_converting.lock().unwrap();
                let can_convert = self.pdf_path.is_some() && !is_converting;
                let status = self.status.lock().unwrap().clone();

                if is_converting {
                     ui.add(egui::ProgressBar::new(status.progress)
                        .show_percentage()
                        .animate(true)
                        .desired_width(ui.available_width()));
                     ui.add_space(10.0);
                     ui.label(egui::RichText::new(&status.message).small().color(egui::Color32::GRAY));
                
                } else if status.is_success {
                    // Estado de Sucesso
                    egui::Frame::none()
                        .inner_margin(10.0)
                        .fill(egui::Color32::from_rgb(230, 255, 230)) // Fundo verde claro
                        .rounding(8.0)
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.vertical_centered(|ui| {
                                ui.label(egui::RichText::new("✅ Conversão Concluída!").color(egui::Color32::BLACK).strong());
                            });
                        });
                        
                    ui.add_space(20.0);
                    
                    // Botão Mostrar na Pasta
                    let btn = egui::Button::new(egui::RichText::new("📂 Abrir Pasta do Arquivo").size(16.0))
                        .min_size(egui::vec2(200.0, 45.0));
                    
                    if ui.add(btn).clicked() {
                         if let Some(path) = &self.output_path {
                             // Abre o Finder selecionando o arquivo
                             let _ = Command::new("open")
                                 .arg("-R")
                                 .arg(path)
                                 .spawn();
                         }
                    }
                    
                    ui.add_space(10.0);
                    if ui.button("Converter outro arquivo").clicked() {
                        self.pdf_path = None;
                        self.output_path = None;
                        let mut s = self.status.lock().unwrap();
                        s.is_success = false;
                        s.message = String::new();
                    }

                } else {
                    // Estado Inicial ou Erro
                    if status.is_error {
                        ui.label(egui::RichText::new(&status.message).color(egui::Color32::RED));
                        ui.add_space(10.0);
                    }

                    // Botão Converter
                    let btn = egui::Button::new(egui::RichText::new("Converter para Keynote").size(18.0).strong())
                        .min_size(egui::vec2(ui.available_width(), 50.0))
                        .fill(if can_convert { egui::Color32::from_rgb(0, 122, 255) } else { egui::Color32::from_gray(200) });
                    
                    if ui.add_enabled(can_convert, btn).clicked() {
                         // Define saída automática
                        if self.output_path.is_none() {
                             if let Some(path) = &self.pdf_path {
                                let mut output = path.clone();
                                output.set_extension("key");
                                self.output_path = Some(output);
                             }
                        }
                        self.start_conversion(ctx.clone());
                    }
                }
            });
        });
    }
}

impl Pdf2KeyApp {
    fn select_pdf(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .pick_file()
        {
            self.pdf_path = Some(path.clone());
            // Reseta status
            let mut status = self.status.lock().unwrap();
            status.message = String::new();
            status.is_error = false;
            status.is_success = false;
            status.progress = 0.0;
        }
    }

    fn start_conversion(&mut self, ctx: egui::Context) {
        let pdf_path = self.pdf_path.clone().unwrap();
        let output_path = self.output_path.clone().unwrap();
        let status = Arc::clone(&self.status);
        let is_converting = Arc::clone(&self.is_converting);
        
        *is_converting.lock().unwrap() = true;
        
        {
            let mut s = status.lock().unwrap();
            s.is_error = false;
            s.is_success = false;
            s.message = "Inicializando...".to_string();
            s.progress = 0.0;
        }
        
        thread::spawn(move || {
            let result = convert_pdf_to_keynote(&pdf_path, &output_path, &status, &ctx);
            
            *is_converting.lock().unwrap() = false;
            
            let mut status_guard = status.lock().unwrap();
            match result {
                Ok(_) => {
                    status_guard.message = "Concluído!".to_string();
                    status_guard.progress = 1.0;
                    status_guard.is_error = false;
                    status_guard.is_success = true;
                }
                Err(e) => {
                    status_guard.message = format!("Erro: {}", e);
                    status_guard.is_error = true;
                    // Se der erro, não mostra botão de sucesso
                    status_guard.is_success = false; 
                }
            }
            ctx.request_repaint();
        });
    }
}

fn convert_pdf_to_keynote(
    pdf_path: &PathBuf,
    output_path: &PathBuf,
    status: &Arc<Mutex<AppStatus>>,
    ctx: &egui::Context,
) -> Result<()> {
    println!("\n========================================");
    println!("[PDF2Key] Iniciando conversão...");
    
    // Configura caminho temporário
    let temp_ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos();
    let temp_dir_path = PathBuf::from(format!("/tmp/pdf2key_{}", temp_ts));
    std::fs::create_dir_all(&temp_dir_path)?;
    println!("[PDF] Temp dir: {:?}", temp_dir_path);
    
    {
        let mut s = status.lock().unwrap();
        s.message = "Renderizando páginas...".to_string();
        s.progress = 0.1;
    }
    ctx.request_repaint();

    // Carrega PDFium
    let processor = pdf_processor::PdfProcessor::new()?;
    
    // Renderiza
    let images = processor.render_pages(pdf_path, 200)?;
    let total_pages = images.len();
    
    let mut image_paths = Vec::new();
    
    // Salva imagens
    for (i, img) in images.iter().enumerate() {
        let progress = 0.2 + (0.5 * (i as f32 / total_pages as f32));
        {
            let mut s = status.lock().unwrap();
            s.message = format!("Processando página {} de {}...", i + 1, total_pages);
            s.progress = progress;
        }
        ctx.request_repaint();
        
        let img_path = temp_dir_path.join(format!("slide_{:04}.png", i));
        img.save_with_format(&img_path, ImageFormat::Png)?;
        image_paths.push(img_path);
    }
    
    {
        let mut s = status.lock().unwrap();
        s.message = "Criando apresentação no Keynote...".to_string();
        s.progress = 0.8;
    }
    ctx.request_repaint();
    
    // Gera Keynote
    let mut builder = keynote::KeynoteBuilder::new();
    for path in &image_paths {
        builder.add_slide(path);
    }
    
    builder.build(output_path)?;
    
    // Tenta limpar (sem falhar)
    let _ = std::fs::remove_dir_all(&temp_dir_path);
    
    println!("========================================\n");
    Ok(())
}
