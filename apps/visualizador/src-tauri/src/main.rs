// Sem janela de console no Windows na versão de produção.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    vidacor_visualizador_app::executar();
}
