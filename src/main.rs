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

// Paleta de cores premium (Dark Theme First)
#[allow(dead_code)]
struct AppColors;

impl AppColors {
    // Fundo Principal (Deep Blue/Black)
    const BG_MAIN: egui::Color32 = egui::Color32::from_rgb(13, 17, 23); // GitHub Dark Dimmed style
    
    // Cores primárias (Electric Blue)
    const PRIMARY: egui::Color32 = egui::Color32::from_rgb(56, 189, 248); // Light Blue 400
    const PRIMARY_HOVER: egui::Color32 = egui::Color32::from_rgb(14, 165, 233); // Sky 500
    const PRIMARY_ACTIVE: egui::Color32 = egui::Color32::from_rgb(2, 132, 199); // Sky 600
    
    // Sucesso (Neon Green)
    const SUCCESS: egui::Color32 = egui::Color32::from_rgb(74, 222, 128); // Green 400
    const SUCCESS_BG: egui::Color32 = egui::Color32::from_rgb(20, 83, 45); // Green 900
    
    // Erro (Soft Red)
    const ERROR: egui::Color32 = egui::Color32::from_rgb(248, 113, 113);
    const ERROR_BG: egui::Color32 = egui::Color32::from_rgb(69, 10, 10);
    
    // Neutros
    const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(241, 245, 249); // Slate 100
    const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(148, 163, 184); // Slate 400
    
    const CARD_BG: egui::Color32 = egui::Color32::from_rgb(30, 41, 59); // Slate 800
    const CARD_BORDER: egui::Color32 = egui::Color32::from_rgb(51, 65, 85); // Slate 700
    const CARD_BORDER_HOVER: egui::Color32 = egui::Color32::from_rgb(71, 85, 105); // Slate 600
    
    const PROGRESS_BG: egui::Color32 = egui::Color32::from_rgb(51, 65, 85);
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 520.0])
            .with_min_inner_size([550.0, 480.0])
            .with_title("PDF2Key")
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "PDF2Key",
        options,
        Box::new(|cc| {
            let mut style = (*cc.egui_ctx.style()).clone();
            
            // Forçar Dark Mode
            style.visuals = egui::Visuals::dark();
            style.visuals.window_fill = AppColors::BG_MAIN;
            style.visuals.panel_fill = AppColors::BG_MAIN;
            
            // Espaçamento e Layout
            style.spacing.item_spacing = egui::vec2(16.0, 16.0);
            style.spacing.button_padding = egui::vec2(24.0, 16.0);
            
            // Cores Globais
            style.visuals.widgets.noninteractive.fg_stroke.color = AppColors::TEXT_PRIMARY;
            style.visuals.hyperlink_color = AppColors::PRIMARY;
            
            // Fontes
            style.text_styles.insert(
                egui::TextStyle::Heading,
                egui::FontId::new(32.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(16.0, egui::FontFamily::Proportional),
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
            // Removemos o scroll e ajustamos as margens para um fit perfeito
            egui::Frame::none()
                .fill(ui.visuals().window_fill()) 
                .inner_margin(20.0) // Reduzi um pouco a margem externa
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.set_max_width(500.0);
                        ui.add_space(10.0); // Espaço menor no topo
                        
                        // Header
                        ui.label(
                            egui::RichText::new("📄 PDF2Key")
                                .size(36.0) // Leve redução
                                .color(AppColors::PRIMARY)
                                .strong()
                        );
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new("Transforme seus PDFs em Keynote rapidamente.")
                                .color(AppColors::TEXT_SECONDARY)
                        );
                        
                        ui.add_space(24.0); // Reduzi de 40.0 para 24.0
                        
                        // Estados
                        let is_converting = *self.is_converting.lock().unwrap();
                        let status = self.status.lock().unwrap().clone();
                        let has_file = self.pdf_path.is_some();
                        
                        // --- CARD PRINCIPAL ---
                        let card_color = if is_converting {
                             AppColors::CARD_BG
                        } else if status.is_success {
                             AppColors::SUCCESS_BG
                        } else {
                             AppColors::CARD_BG
                        };
                        
                        let border_color = if is_converting {
                            AppColors::PRIMARY
                        } else if status.is_success {
                            AppColors::SUCCESS
                        } else if has_file {
                            AppColors::PRIMARY
                        } else {
                            AppColors::CARD_BORDER
                        };

                        let card_response = egui::Frame::group(ui.style())
                            .inner_margin(30.0) // Reduzi de 40.0 para 30.0
                            .rounding(16.0)
                            .stroke(egui::Stroke::new(2.0, border_color))
                            .fill(card_color)
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.set_min_height(140.0);
                                
                                ui.vertical_centered(|ui| {
                                    if is_converting {
                                        ui.spinner();
                                        ui.add_space(16.0);
                                        
                                        // Custom Progress Bar Dark
                                        let w = ui.available_width();
                                        let h = 8.0;
                                        let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
                                        
                                        ui.painter().rect_filled(rect, 4.0, AppColors::PROGRESS_BG);
                                        if status.progress > 0.0 {
                                            let fill_w = w * status.progress;
                                            let fill_rect = egui::Rect::from_min_size(rect.min, egui::vec2(fill_w, h));
                                            ui.painter().rect_filled(fill_rect, 4.0, AppColors::PRIMARY);
                                        }
                                        
                                        ui.add_space(12.0);
                                        ui.label(egui::RichText::new(&status.message).color(AppColors::TEXT_SECONDARY));
                                        
                                    } else if status.is_success {
                                        ui.label(egui::RichText::new("🚀 Sucesso!").size(24.0).strong().color(AppColors::SUCCESS));
                                        ui.add_space(8.0);
                                        if let Some(path) = &self.output_path {
                                            ui.label(
                                                egui::RichText::new(path.file_name().unwrap_or_default().to_string_lossy())
                                                    .monospace()
                                                    .color(AppColors::TEXT_PRIMARY)
                                            );
                                        }
                                    } else if let Some(path) = &self.pdf_path {
                                        ui.label(egui::RichText::new("📄 Arquivo Pronto").size(20.0).strong().color(AppColors::PRIMARY));
                                        ui.add_space(8.0);
                                        ui.label(
                                            egui::RichText::new(path.file_name().unwrap_or_default().to_string_lossy())
                                                .size(16.0)
                                                .color(AppColors::TEXT_PRIMARY)
                                        );
                                        ui.add_space(12.0);
                                        ui.label(egui::RichText::new("Clique para alterar").size(12.0).color(AppColors::TEXT_SECONDARY));
                                    } else {
                                        ui.label(egui::RichText::new("📂").size(48.0).color(AppColors::TEXT_SECONDARY));
                                        ui.add_space(16.0);
                                        ui.label(egui::RichText::new("Clique para selecionar um PDF").size(18.0).strong().color(AppColors::TEXT_PRIMARY));
                                    }
                                });
                            }).response;

                        if !is_converting && !status.is_success {
                            if card_response.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            if card_response.interact(egui::Sense::click()).clicked() {
                                self.select_pdf();
                            }
                        }

                        if status.is_error {
                            ui.add_space(16.0);
                            egui::Frame::none()
                                .fill(AppColors::ERROR_BG)
                                .inner_margin(12.0)
                                .rounding(8.0)
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new(format!("Erro: {}", status.message)).color(AppColors::ERROR));
                                });
                        }

                        ui.add_space(32.0); // Reduzi margem bottom

                        // --- ACTIONS ---
                        if !is_converting {
                            if status.is_success {
                                ui.horizontal(|ui| {
                                    ui.columns(2, |cols| {
                                        cols[0].vertical_centered_justified(|ui| {
                                            let btn = egui::Button::new(
                                                egui::RichText::new("📂 Abrir Pasta").strong().color(egui::Color32::BLACK)
                                            )
                                            .min_size(egui::vec2(0.0, 50.0))
                                            .fill(AppColors::PRIMARY)
                                            .rounding(10.0);
                                            
                                            if ui.add(btn).clicked() {
                                                if let Some(path) = &self.output_path {
                                                     let _ = Command::new("open").arg("-R").arg(path).spawn();
                                                }
                                            }
                                        });
                                        
                                        cols[1].vertical_centered_justified(|ui| {
                                            let btn = egui::Button::new(
                                                egui::RichText::new("🔄 Novo").strong().color(AppColors::TEXT_PRIMARY)
                                            )
                                            .min_size(egui::vec2(0.0, 50.0))
                                            .fill(egui::Color32::TRANSPARENT)
                                            .stroke(egui::Stroke::new(1.0, AppColors::CARD_BORDER))
                                            .rounding(10.0);
                                            
                                            if ui.add(btn).clicked() {
                                                self.pdf_path = None;
                                                self.output_path = None;
                                                let mut s = self.status.lock().unwrap();
                                                s.is_success = false;
                                                s.message = String::new();
                                            }
                                        });
                                    });
                                });
                            } else {
                                let btn_text = if has_file { "Converter agora" } else { "Selecione um arquivo" };
                                let btn_color = if has_file { AppColors::PRIMARY } else { AppColors::CARD_BORDER };
                                let txt_color = if has_file { egui::Color32::BLACK } else { AppColors::TEXT_SECONDARY };
                                
                                let btn = egui::Button::new(
                                    egui::RichText::new(btn_text).size(18.0).strong().color(txt_color)
                                )
                                .min_size(egui::vec2(ui.available_width(), 56.0))
                                .fill(btn_color)
                                .rounding(12.0);
                                
                                if ui.add_enabled(has_file, btn).clicked() {
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
                        }
                    });
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
