//! Módulo para controle do Apple Keynote via AppleScript
//! Cria apresentações editáveis diretamente no Keynote

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Controla o Keynote via AppleScript para criar apresentações
pub struct KeynoteBuilder {
    slide_images: Vec<String>,
}

impl KeynoteBuilder {
    /// Cria um novo builder para apresentações Keynote
    pub fn new() -> Self {
        Self {
            slide_images: Vec::new(),
        }
    }

    /// Adiciona uma imagem como um novo slide
    pub fn add_slide(&mut self, image_path: &Path) {
        self.slide_images.push(image_path.to_string_lossy().to_string());
    }

    /// Constrói e salva a apresentação no Keynote
    pub fn build(&self, output_path: &Path) -> Result<()> {
        if self.slide_images.is_empty() {
            anyhow::bail!("Nenhum slide foi adicionado");
        }

        let output_path_str = output_path.to_string_lossy().to_string();
        
        println!("[Keynote] Criando apresentação...");

        // Gera lista de imagens para o AppleScript
        let image_list: Vec<String> = self.slide_images
            .iter()
            .map(|p| format!("\"{}\"", p))
            .collect();
        let image_list_str = image_list.join(", ");

        // AppleScript robusto com tratamento de alias
        let applescript = format!(
            r#"
set imageList to {{{image_list}}}
set outputPath to "{output_path}"

tell application "Keynote"
    -- activate -- Removido para não trazer para frente
    set theDoc to make new document
    
    set slideWidth to width of theDoc
    set slideHeight to height of theDoc
    
    repeat with i from 1 to count of imageList
        set imagePath to item i of imageList
        
        -- Converte para alias para garantir acesso
        set imageFile to (POSIX file imagePath) as alias
        
        if i is 1 then
            set currentSlide to slide 1 of theDoc
        else
            set currentSlide to make new slide at end of slides of theDoc
        end if
        
        tell currentSlide
            set theImage to make new image with properties {{file:imageFile}}
            set width of theImage to slideWidth
            set height of theImage to slideHeight
            set position of theImage to {{0, 0}}
        end tell
    end repeat
    
    save theDoc in POSIX file outputPath
    -- close theDoc
end tell
"#,
            image_list = image_list_str,
            output_path = output_path_str
        );

        println!("[Keynote] Executando AppleScript...");

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&applescript)
            .output()
            .context("Falha ao executar osascript")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("[Keynote] ERRO: {}", stderr);
            anyhow::bail!("Erro no Keynote (Verifique permissões de acesso): {}", stderr);
        }

        println!("[Keynote] ✓ Apresentação criada com sucesso!");
        Ok(())
    }
}
