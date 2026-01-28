//! Módulo para processamento de PDFs
//! Usa pdfium-render para renderizar páginas como imagens

use anyhow::{Context, Result};
use image::{DynamicImage, RgbaImage};
use pdfium_render::prelude::*;
use std::path::Path;

/// Carrega e renderiza todas as páginas de um PDF como imagens
pub struct PdfProcessor {
    pdfium: Pdfium,
}

impl PdfProcessor {
    /// Cria uma nova instância do processador de PDF
    pub fn new() -> Result<Self> {
        // Tenta carregar a biblioteca pdfium de vários locais
        let pdfium = Pdfium::new(
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./lib/"))
                .or_else(|_| Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./")))
                .or_else(|_| Pdfium::bind_to_system_library())
                .context("Não foi possível encontrar a biblioteca PDFium. Verifique se lib/libpdfium.dylib existe.")?,
        );
        
        Ok(Self { pdfium })
    }

    /// Renderiza todas as páginas do PDF como imagens
    /// 
    /// # Arguments
    /// * `pdf_path` - Caminho para o arquivo PDF
    /// * `dpi` - Resolução de renderização (recomendado: 150-300)
    /// 
    /// # Returns
    /// Vetor de imagens, uma para cada página
    pub fn render_pages(&self, pdf_path: &Path, dpi: u16) -> Result<Vec<DynamicImage>> {
        let document = self.pdfium
            .load_pdf_from_file(pdf_path, None)
            .context("Falha ao abrir o arquivo PDF")?;

        let pages = document.pages();
        let page_count = pages.len();
        let mut images = Vec::with_capacity(page_count as usize);

        for (index, page) in pages.iter().enumerate() {
            let render_config = PdfRenderConfig::new()
                .set_target_width(
                    (page.width().value * dpi as f32 / 72.0) as i32
                )
                .set_maximum_height(
                    (page.height().value * dpi as f32 / 72.0) as i32
                );

            let bitmap = page
                .render_with_config(&render_config)
                .context(format!("Falha ao renderizar página {}", index + 1))?;

            let image = bitmap
                .as_image();

            // Converte para DynamicImage
            let rgba_image: RgbaImage = image.into_rgba8();
            images.push(DynamicImage::ImageRgba8(rgba_image));
        }

        Ok(images)
    }

    /// Retorna o número de páginas no PDF
    pub fn page_count(&self, pdf_path: &Path) -> Result<usize> {
        let document = self.pdfium
            .load_pdf_from_file(pdf_path, None)
            .context("Falha ao abrir o arquivo PDF")?;
        
        Ok(document.pages().len() as usize)
    }
}
