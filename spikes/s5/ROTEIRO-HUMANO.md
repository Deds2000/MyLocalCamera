# ROTEIRO-HUMANO — Spike S5 (6 execuções × 10 min)

Objetivo: coletar as medições dos modos **A** (WebCodecs) e **B** (JPEG do Rust)
em 100%, 150% e 200% de escala do Windows.

O script faz tudo sozinho: sobe o app, amostra CPU/memória/GPU a cada 1 s e
encerra o app no fim. Você só executa os comandos abaixo e muda a escala do
Windows entre os grupos.

## Preparo (uma vez)
1. Feche apps pesados (jogos, navegador com muitas abas, editores de vídeo)
   para reduzir o ruído da medição.
2. Deixar o PC na tomada (ou plano de energia "melhor desempenho").
3. Janela do app visível e em frente (não minimizada) durante cada execução.

## Como mudar a escala do Windows (entre os grupos)
Configurações → Sistema → Tela → "Escala e layout".

- Mude 100% → 150% → 200% (de volta se quiser).
- O Windows pode pedir para **sair da conta e entrar de novo** para a mudança
  valer em todos os apps. Faça só se ele pedir ou se a janela do app não
  refletir a nova escala.
- O script **NÃO** muda a escala — o parâmetro `-Dpi` é só um rótulo do CSV.

## Comandos (6, nesta ordem, a partir da raiz do repositório)

```powershell
.\spikes\s5\run-s5.ps1 -Mode A -Dpi 100    # ~10 min
.\spikes\s5\run-s5.ps1 -Mode B -Dpi 100    # ~10 min

# → mude a escala do Windows para 150% (veja acima)

.\spikes\s5\run-s5.ps1 -Mode A -Dpi 150    # ~10 min
.\spikes\s5\run-s5.ps1 -Mode B -Dpi 150    # ~10 min

# → mude a escala do Windows para 200% (veja acima)

.\spikes\s5\run-s5.ps1 -Mode A -Dpi 200    # ~10 min
.\spikes\s5\run-s5.ps1 -Mode B -Dpi 200    # ~10 min
```

Cada execução dura ~11 min (10 min de medição + subida/encerramento).
No fim, o script imprime uma linha `RESUMO s5 | ...` e encerra tudo sozinho.

## O que devolver ao agente
- As **6 linhas `RESUMO`** (uma por execução), copiadas do console; e/ou
- Os **6 CSVs** de `docs\spikes\tmp\` (`s5-A-100.csv`, `s5-B-100.csv`, ...);
- Observações rápidas: nitidez em cada escala, travamentos/quadros perdidos
  vistos a olho, se algo pareceu estranho.

## Se algo falhar
- Se a execução terminar com `gpu=n/a ... [erro]`, copie a linha inteira ao agente.
- Se o app ficar aberto depois da execução, feche a janela "S5 spike" e avise.
- Não mude a escala do Windows no meio de uma execução (o rótulo não combina
  mais com a medição).
