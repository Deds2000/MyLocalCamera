// Gerador de quadros NV12 1080p sintéticos para o spike S2.
//
// O padrão é visivelmente diferente do padrão de teste do sample S1
// (gradiente azul rolante) para o humano confirmar de olho que o quadro que
// aparece no app consumidor vem do servidor do spike:
//   - fundo verde;
//   - barra branca vertical de 64 px que varre a largura em 1 s (64 px/quadro);
//   - a posição depende de `seq`, então queda de quadros aparece como "salto".
//
// Valores NV12 (BT.601 limitado): verde = (Y 145, U 54, V 34),
// branco = (Y 235, U 128, V 128).

use crate::protocol::{FRAME_H, FRAME_W, NV12_SIZE};

pub fn fill_nv12(dst: &mut [u8], seq: u64) {
    debug_assert_eq!(dst.len(), NV12_SIZE);
    let w = FRAME_W as usize;
    let h = FRAME_H as usize;
    let y_size = w * h;

    // Plano Y: fundo verde.
    dst[..y_size].fill(145);
    // Plano UV (interleaved): verde.
    for i in (y_size..dst.len()).step_by(2) {
        dst[i] = 54;
        dst[i + 1] = 34;
    }

    // Barra branca: x0 move 64 px por quadro, com wrap.
    let x0 = (seq as usize * 64) % w;
    let bw = 64usize;
    let y = &mut dst[..y_size];
    for r in 0..h {
        let row = &mut y[r * w..(r + 1) * w];
        for x in 0..bw {
            row[(x0 + x) % w] = 235;
        }
    }
    // UV da barra: cada par U,V cobre 2 colunas de Y.
    let uv = &mut dst[y_size..];
    for ur in 0..h / 2 {
        let uvrow = &mut uv[ur * w..(ur + 1) * w];
        for x in 0..bw {
            let xx = (x0 + x) % w;
            if xx % 2 == 0 {
                uvrow[xx] = 128;
                uvrow[xx + 1] = 128;
            }
        }
    }
}
