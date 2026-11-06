"""Gera as imagens de exame ilustrativas usadas nos mockups.

As imagens vêm dos arquivos DICOM de exemplo distribuídos com a biblioteca
pydicom (pasta pydicom/data/test_files). Não há dado de paciente real da
VidaCor aqui.

Uso:
    python -m venv .venv && .venv/bin/pip install pydicom pylibjpeg pylibjpeg-openjpeg pylibjpeg-libjpeg numpy pillow
    .venv/bin/python -I tools/imagens-dicom/extrair_exemplos.py design/assets/imagens-exame
"""
import os
import sys

import numpy as np
import pydicom
from PIL import Image
from pydicom.pixels import convert_color_space

SAIDA = sys.argv[1] if len(sys.argv) > 1 else "."
DADOS = os.path.join(os.path.dirname(pydicom.__file__), "data", "test_files")


def ler(nome):
    return pydicom.dcmread(os.path.join(DADOS, nome))


def janela(hu, centro, largura):
    return (np.clip((hu - (centro - largura / 2)) / largura, 0, 1) * 255).astype("uint8")


def tc_cranio():
    # O arquivo tem valores de osso "dobrados" (pixel representation incoerente):
    # valores < -1200 que não são o preenchimento -2000 são osso.
    a = ler("J2K_pixelrep_mismatch.dcm").pixel_array.astype(np.int32)
    x = np.clip((a + 5) / 90, 0, 1) * 255
    osso = (a < -1200) & (a != -2000)
    x = np.where(a <= -900, np.where(osso, 255, 0), x)
    Image.fromarray(x.astype("uint8")).save(os.path.join(SAIDA, "tc-cranio-real.png"))


def tc_torax():
    ds = ler("CT_small.dcm")
    hu = ds.pixel_array * float(ds.get("RescaleSlope", 1)) + float(ds.get("RescaleIntercept", 0))
    Image.fromarray(janela(hu, 40, 400)).save(os.path.join(SAIDA, "tc-torax-real.png"))


def mono(nome, saida):
    a = ler(nome).pixel_array.astype(float)
    lo, hi = np.percentile(a, 1), np.percentile(a, 99.5)
    Image.fromarray((np.clip((a - lo) / (hi - lo + 1e-6), 0, 1) * 255).astype("uint8")).save(os.path.join(SAIDA, saida))


def doppler():
    ds = ler("examples_rgb_color.dcm")
    a = ds.pixel_array
    if ds.PhotometricInterpretation.startswith("YBR"):
        a = convert_color_space(a, ds.PhotometricInterpretation, "RGB")
    Image.fromarray(a).crop((0, 40, 320, 195)).resize((640, 310), Image.LANCZOS).save(
        os.path.join(SAIDA, "us-doppler-real.png"))


if __name__ == "__main__":
    os.makedirs(SAIDA, exist_ok=True)
    tc_cranio()
    tc_torax()
    mono("examples_overlay.dcm", "rm-abdome-real.png")
    mono("JPEG2000.dcm", "mn-cintilografia-real.png")
    doppler()
    print("ok")
