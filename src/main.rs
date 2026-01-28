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
use tempfile::TempDir;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([500.0, 400.0])
            .with_min_inner_size([400.0, 300.0])
            .with_title("PDF2Key"),
        ..Default::default()
    };

    eframe::run_native(
        "PDF2Key",
        options,
        Box::new(|cc| {
            // Configura estilo visual
            let mut style = (*cc.egui_ctx.style()).clone();
            style.spacing.item_spacing = egui::vec2(10.0, 10.0);
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
}

impl eframe::App for Pdf2KeyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Força atualização contínua durante conversão
        if *self.is_converting.lock().unwrap() {
            ctx.request_repaint();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                
                // Título
                ui.heading(egui::RichText::new("📄 PDF2Key").size(32.0));
                ui.label("Converta PDFs para Apple Keynote");
                
                ui.add_space(30.0);
                
                // Seleção de arquivo PDF
                ui.horizontal(|ui| {
                    ui.label("PDF:");
                    
                    let pdf_text = self.pdf_path
                        .as_ref()
                        .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
                        .unwrap_or_else(|| "Nenhum arquivo selecionado".to_string());
                    
                    ui.label(egui::RichText::new(&pdf_text).monospace());
                    
                    if ui.button("📂 Selecionar").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("PDF", &["pdf"])
                            .pick_file()
                        {
                            self.pdf_path = Some(path.clone());
                            // Auto-sugere nome de saída
                            let mut output = path.clone();
                            output.set_extension("key");
                            self.output_path = Some(output);
                        }
                    }
                });
                
                ui.add_space(10.0);
                
                // Seleção de destino
                ui.horizontal(|ui| {
                    ui.label("Salvar:");
                    
                    let output_text = self.output_path
                        .as_ref()
                        .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
                        .unwrap_or_else(|| "Escolha onde salvar".to_string());
                    
                    ui.label(egui::RichText::new(&output_text).monospace());
                    
                    if ui.button("📂 Escolher").clicked() {
                        let default_name = self.pdf_path
                            .as_ref()
                            .and_then(|p| p.file_stem())
                            .map(|s| format!("{}.key", s.to_string_lossy()))
                            .unwrap_or_else(|| "apresentacao.key".to_string());
                        
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Keynote", &["key"])
                            .set_file_name(&default_name)
                            .save_file()
                        {
                            self.output_path = Some(path);
                        }
                    }
                });
                
                ui.add_space(30.0);
                
                // Botão de conversão
                let is_converting = *self.is_converting.lock().unwrap();
                let can_convert = self.pdf_path.is_some() && self.output_path.is_some() && !is_converting;
                
                ui.add_enabled_ui(can_convert, |ui| {
                    if ui.add_sized(
                        [200.0, 40.0],
                        egui::Button::new(egui::RichText::new("🚀 Converter").size(18.0))
                    ).clicked() {
                        self.start_conversion(ctx.clone());
                    }
                });
                
                ui.add_space(20.0);
                
                // Status e progresso
                let status = self.status.lock().unwrap().clone();
                
                if !status.message.is_empty() {
                    let color = if status.is_error {
                        egui::Color32::from_rgb(220, 50, 50)
                    } else {
                        egui::Color32::from_rgb(100, 180, 100)
                    };
                    
                    ui.label(egui::RichText::new(&status.message).color(color));
                }
                
                if is_converting {
                    ui.add(egui::ProgressBar::new(status.progress).show_percentage());
                }
            });
        });
    }
}

impl Pdf2KeyApp {
    fn start_conversion(&mut self, ctx: egui::Context) {
        let pdf_path = self.pdf_path.clone().unwrap();
        let output_path = self.output_path.clone().unwrap();
        let status = Arc::clone(&self.status);
        let is_converting = Arc::clone(&self.is_converting);
        
        *is_converting.lock().unwrap() = true;
        
        thread::spawn(move || {
            let result = convert_pdf_to_keynote(&pdf_path, &output_path, &status, &ctx);
            
            *is_converting.lock().unwrap() = false;
            
            let mut status_guard = status.lock().unwrap();
            match result {
                Ok(_) => {
                    status_guard.message = "✅ Conversão concluída com sucesso!".to_string();
                    status_guard.progress = 1.0;
                    status_guard.is_error = false;
                }
                Err(e) => {
                    status_guard.message = format!("❌ Erro: {}", e);
                    status_guard.is_error = true;
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
    println!("[PDF2Key] PDF: {:?}", pdf_path);
    println!("[PDF2Key] Saída: {:?}", output_path);
    println!("========================================\n");
    
    // Atualiza status
    {
        let mut s = status.lock().unwrap();
        s.message = "Carregando PDF...".to_string();
        s.progress = 0.1;
        s.is_error = false;
    }
    ctx.request_repaint();
    
    // Cria processador de PDF
    println!("[PDF] Carregando biblioteca PDFium...");
    let processor = pdf_processor::PdfProcessor::new()?;
    println!("[PDF] ✓ PDFium carregado com sucesso!");
    
    // Cria diretório temporário acessível (use /tmp para evitar problemas de sandbox)
    let temp_ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos();
    let temp_dir_path = PathBuf::from(format!("/tmp/pdf2key_{}", temp_ts));
    std::fs::create_dir_all(&temp_dir_path)?;
    println!("[PDF] Diretório temporário: {:?}", temp_dir_path);
    
    {
        let mut s = status.lock().unwrap();
        s.message = "Renderizando páginas...".to_string();
        s.progress = 0.2;
    }
    ctx.request_repaint();
    
    // Renderiza páginas
    println!("[PDF] Renderizando páginas (200 DPI)...");
    let images = processor.render_pages(pdf_path, 200)?;
    let total_pages = images.len();
    println!("[PDF] ✓ {} páginas renderizadas!", total_pages);
    
    // Salva imagens temporárias
    println!("[PDF] Salvando imagens temporárias...");
    let mut image_paths = Vec::new();
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
        println!("[PDF] ✓ Página {} salva: {:?}", i + 1, img_path);
        image_paths.push(img_path);
    }
    
    {
        let mut s = status.lock().unwrap();
        s.message = "Criando apresentação no Keynote...".to_string();
        s.progress = 0.8;
    }
    ctx.request_repaint();
    
    // Cria apresentação no Keynote
    println!("\n[Keynote] Iniciando criação da apresentação...");
    let mut builder = keynote::KeynoteBuilder::new();
    for path in &image_paths {
        builder.add_slide(path);
    }
    
    builder.build(output_path)?;
    
    println!("\n========================================");
    println!("[PDF2Key] ✓ CONVERSÃO CONCLUÍDA!");
    println!("[PDF2Key] Arquivo: {:?}", output_path);
    println!("========================================\n");
    
    Ok(())
}

