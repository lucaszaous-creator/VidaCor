"""Gera os exames DICOM sintéticos usados nos testes do visualizador (crates/visualizador).

Tudo é inventado: pacientes fictícios, imagens desenhadas por fórmula e UIDs fixos sob a raiz
de exemplo 1.2.826.0.1.3680043.10.1999 (não é UID de aparelho). Nenhum exame real entra no
repositório (docs/31, VIS-S5).

Cada arquivo cobre uma forma de gravação usada por aparelhos de imagem:
  ct_lossless_*.dcm   TC 16 bits com sinal, JPEG Lossless SV1 (como o Canon Aquilion), 3 cortes
                      com InstanceNumber fora da ordem da posição, para testar a ordenação
  ct_sem_compressao   TC 16 bits sem sinal com RescaleIntercept -1024, Explicit VR Little Endian
  cr_monochrome1      Raio-X 8 bits MONOCHROME1 (branco é zero), Implicit VR Little Endian
  mg_jpeg2000         Mamografia 12 bits em JPEG 2000 sem perda
  mr_rle              Ressonância 16 bits em RLE
  mr_jpegls           Ressonância 16 bits em JPEG-LS sem perda
  us_cine_rgb         Ultrassom colorido com 4 quadros, JPEG Baseline (YBR_FULL_422)
  us_paleta           Ultrassom PALETTE COLOR 8 bits com tabela de 256 cores de 16 bits
  us_ybr_planar       Ultrassom YBR_FULL sem compressão com os planos separados (PlanarConfiguration 1)
  outro_paciente      TC de outro paciente fictício com acento (Latin-1), para o agrupamento e o
                      bloqueio VIS-S1
  nao_dicom.txt       arquivo que não é DICOM
  truncado.dcm        DICOM cortado no meio dos pixels
  exame.zip           todos os acima dentro de um .zip, com subpasta

Uso (versões exatas):
    python -m venv .venv && .venv/bin/pip install pydicom==3.0.2 numpy python-gdcm==3.2.6
    .venv/bin/python -I tools/imagens-dicom/gerar_amostras_teste.py crates/visualizador/tests/dados
"""
import os
import sys
import zipfile

import gdcm
import numpy as np
from pydicom.dataset import Dataset, FileMetaDataset
from pydicom.uid import (
    ExplicitVRLittleEndian,
    ImplicitVRLittleEndian,
)

SAIDA = sys.argv[1]
RAIZ = "1.2.826.0.1.3680043.10.1999"
CT, CR, MG, MR, US = (
    "1.2.840.10008.5.1.4.1.1.2",
    "1.2.840.10008.5.1.4.1.1.1",
    "1.2.840.10008.5.1.4.1.1.1.2",
    "1.2.840.10008.5.1.4.1.1.4",
    "1.2.840.10008.5.1.4.1.1.3.1",
)
TS_GDCM = {
    "jpeg_lossless": gdcm.TransferSyntax.JPEGLosslessProcess14_1,
    "jpeg2000": gdcm.TransferSyntax.JPEG2000Lossless,
    "rle": gdcm.TransferSyntax.RLELossless,
    "jpegls": gdcm.TransferSyntax.JPEGLSLossless,
    "jpeg_baseline": gdcm.TransferSyntax.JPEGBaselineProcess1,
}


def paciente(ds, nome, pid, nascimento, sexo):
    ds.PatientName = nome
    ds.PatientID = pid
    ds.PatientBirthDate = nascimento
    ds.PatientSex = sexo


def base(sop_class, estudo, serie, instancia, modalidade):
    ds = Dataset()
    ds.file_meta = FileMetaDataset()
    ds.file_meta.MediaStorageSOPClassUID = sop_class
    ds.file_meta.MediaStorageSOPInstanceUID = f"{RAIZ}.{estudo}.{serie}.{instancia}"
    ds.file_meta.TransferSyntaxUID = ExplicitVRLittleEndian
    ds.file_meta.ImplementationClassUID = f"{RAIZ}.0"
    ds.SOPClassUID = sop_class
    ds.SOPInstanceUID = ds.file_meta.MediaStorageSOPInstanceUID
    ds.StudyInstanceUID = f"{RAIZ}.{estudo}"
    ds.SeriesInstanceUID = f"{RAIZ}.{estudo}.{serie}"
    ds.Modality = modalidade
    ds.StudyDate = "20260101"
    ds.StudyTime = "093000"
    ds.AccessionNumber = f"TESTE{estudo:04d}"
    ds.ReferringPhysicianName = "FICTICIO^MEDICO"
    ds.InstitutionName = "CLINICA FICTICIA"
    ds.SeriesNumber = serie
    ds.InstanceNumber = instancia
    paciente(ds, "FICTICIO^ANA MARIA", "TESTE001", "19800101", "F")
    return ds


def pixels_mono(ds, arr, bits_stored, com_sinal, fotometria="MONOCHROME2"):
    ds.SamplesPerPixel = 1
    ds.PhotometricInterpretation = fotometria
    ds.Rows, ds.Columns = arr.shape[-2:]
    ds.BitsAllocated = arr.dtype.itemsize * 8
    ds.BitsStored = bits_stored
    ds.HighBit = bits_stored - 1
    ds.PixelRepresentation = 1 if com_sinal else 0
    ds.PixelData = arr.tobytes()


def fantoma(n, minimo, maximo, deslocamento=0):
    """Degradê horizontal com um quadrado no centro: valores conhecidos para conferir a decodificação."""
    x = np.linspace(minimo, maximo, n)
    arr = np.tile(x, (n, 1))
    arr[n // 4 : 3 * n // 4, n // 4 : 3 * n // 4] = (minimo + maximo) / 2 + deslocamento
    return arr


def comprimir(caminho, ts):
    leitor = gdcm.ImageReader()
    leitor.SetFileName(caminho)
    if not leitor.Read():
        raise SystemExit(f"GDCM não leu {caminho}")
    troca = gdcm.ImageChangeTransferSyntax()
    troca.SetTransferSyntax(gdcm.TransferSyntax(TS_GDCM[ts]))
    troca.SetInput(leitor.GetImage())
    if not troca.Change():
        raise SystemExit(f"GDCM não comprimiu {caminho} em {ts}")
    escritor = gdcm.ImageWriter()
    escritor.SetFileName(caminho)
    escritor.SetFile(leitor.GetFile())
    escritor.SetImage(troca.GetOutput())
    if not escritor.Write():
        raise SystemExit(f"GDCM não gravou {caminho}")


def salvar(ds, nome, ts=None, implicito=False):
    caminho = os.path.join(SAIDA, nome)
    if implicito:
        ds.file_meta.TransferSyntaxUID = ImplicitVRLittleEndian
    ds.save_as(caminho, implicit_vr=implicito, little_endian=True, enforce_file_format=True)
    if ts:
        comprimir(caminho, ts)
    return caminho


def main():
    os.makedirs(SAIDA, exist_ok=True)
    arquivos = []

    # TC 16 bits com sinal em JPEG Lossless; posição z decrescente com o InstanceNumber crescente
    for i, z in ((1, 20.0), (2, 10.0), (3, 0.0)):
        ds = base(CT, 1, 2, i, "CT")
        ds.StudyDescription = "TC CRANIO"
        ds.SeriesDescription = "PARTES MOLES"
        ds.ImageOrientationPatient = [1, 0, 0, 0, 1, 0]
        ds.ImagePositionPatient = [0, 0, z]
        ds.PixelSpacing = [0.5, 0.5]
        ds.SliceThickness = 10
        ds.RescaleIntercept, ds.RescaleSlope = 0, 1
        ds.WindowCenter, ds.WindowWidth = 35, 100
        arr = fantoma(64, -1000, 1000, deslocamento=int(z)).astype(np.int16)
        pixels_mono(ds, arr, 16, True)
        arquivos.append(salvar(ds, f"ct_lossless_{i}.dcm", "jpeg_lossless"))

    # TC sem compressão, sem sinal, com intercept -1024 e duas janelas no cabeçalho
    ds = base(CT, 1, 3, 1, "CT")
    ds.StudyDescription = "TC CRANIO"
    ds.SeriesDescription = "OSSO"
    ds.RescaleIntercept, ds.RescaleSlope = -1024, 1
    ds.WindowCenter, ds.WindowWidth = [500, 40], [2000, 80]
    pixels_mono(ds, fantoma(64, 0, 3000).astype(np.uint16), 12, False)
    arquivos.append(salvar(ds, "ct_sem_compressao.dcm"))

    # Raio-X MONOCHROME1 8 bits, implícito
    ds = base(CR, 2, 1, 1, "CR")
    ds.StudyDescription = "RX TORAX"
    ds.SeriesDescription = "PA"
    pixels_mono(ds, fantoma(64, 0, 255).astype(np.uint8), 8, False, "MONOCHROME1")
    arquivos.append(salvar(ds, "cr_monochrome1.dcm", implicito=True))

    # Mamografia 12 bits em JPEG 2000 sem perda
    ds = base(MG, 3, 1, 1, "MG")
    ds.StudyDescription = "MAMOGRAFIA"
    ds.SeriesDescription = "CC ESQ"
    ds.WindowCenter, ds.WindowWidth = 2048, 4096
    pixels_mono(ds, fantoma(64, 0, 4095).astype(np.uint16), 12, False)
    arquivos.append(salvar(ds, "mg_jpeg2000.dcm", "jpeg2000"))

    # Ressonância em RLE e em JPEG-LS
    for nome, ts, serie in (("mr_rle.dcm", "rle", 1), ("mr_jpegls.dcm", "jpegls", 2)):
        ds = base(MR, 4, serie, 1, "MR")
        ds.StudyDescription = "RM JOELHO"
        ds.SeriesDescription = "T1" if serie == 1 else "T2"
        pixels_mono(ds, fantoma(64, 0, 1200).astype(np.uint16), 16, False)
        arquivos.append(salvar(ds, nome, ts))

    # Ultrassom colorido com 4 quadros em JPEG Baseline
    ds = base(US, 5, 1, 1, "US")
    ds.StudyDescription = "US ABDOME"
    ds.SeriesDescription = "CINE"
    quadros = []
    for q in range(4):
        rgb = np.zeros((64, 64, 3), np.uint8)
        rgb[..., 0] = 200
        rgb[..., 1] = q * 60
        rgb[16:48, 16:48, 2] = 255
        quadros.append(rgb)
    arr = np.stack(quadros)
    ds.NumberOfFrames = 4
    ds.FrameTime = 40
    ds.SamplesPerPixel = 3
    ds.PhotometricInterpretation = "RGB"
    ds.PlanarConfiguration = 0
    ds.Rows, ds.Columns = 64, 64
    ds.BitsAllocated = ds.BitsStored = 8
    ds.HighBit = 7
    ds.PixelRepresentation = 0
    ds.PixelData = arr.tobytes()
    arquivos.append(salvar(ds, "us_cine_rgb.dcm", "jpeg_baseline"))

    # Ultrassom com paleta de cores: índice i vira (i, 255 - i, 128) em 16 bits
    ds = base(US, 5, 2, 1, "US")
    ds.StudyDescription = "US ABDOME"
    ds.SeriesDescription = "PALETA"
    idx = np.tile(np.arange(64, dtype=np.uint8) * 4, (64, 1))
    pixels_mono(ds, idx, 8, False, "PALETTE COLOR")
    ds.RedPaletteColorLookupTableDescriptor = [256, 0, 16]
    ds.GreenPaletteColorLookupTableDescriptor = [256, 0, 16]
    ds.BluePaletteColorLookupTableDescriptor = [256, 0, 16]
    i = np.arange(256, dtype=np.uint16)
    ds.RedPaletteColorLookupTableData = (i * 257).astype("<u2").tobytes()
    ds.GreenPaletteColorLookupTableData = ((255 - i) * 257).astype("<u2").tobytes()
    ds.BluePaletteColorLookupTableData = (np.full(256, 128, np.uint16) * 257).astype("<u2").tobytes()
    arquivos.append(salvar(ds, "us_paleta.dcm"))

    # Ultrassom YBR_FULL com planos separados: Y=76, Cb=85, Cr=255 é vermelho puro
    ds = base(US, 5, 3, 1, "US")
    ds.StudyDescription = "US ABDOME"
    ds.SeriesDescription = "YBR"
    planos = np.stack([np.full((64, 64), v, np.uint8) for v in (76, 85, 255)])
    ds.SamplesPerPixel = 3
    ds.PhotometricInterpretation = "YBR_FULL"
    ds.PlanarConfiguration = 1
    ds.Rows, ds.Columns = 64, 64
    ds.BitsAllocated = ds.BitsStored = 8
    ds.HighBit = 7
    ds.PixelRepresentation = 0
    ds.PixelData = planos.tobytes()
    arquivos.append(salvar(ds, "us_ybr_planar.dcm"))

    # Outro paciente fictício
    ds = base(CT, 6, 1, 1, "CT")
    ds.SpecificCharacterSet = "ISO_IR 100"
    paciente(ds, "FICTICIO^JOÃO PEDRO", "TESTE002", "19750615", "M")
    ds.StudyDescription = "TC ABDOME"
    ds.SeriesDescription = "PORTAL"
    ds.RescaleIntercept, ds.RescaleSlope = 0, 1
    pixels_mono(ds, fantoma(64, -100, 300).astype(np.int16), 16, True)
    arquivos.append(salvar(ds, "outro_paciente.dcm"))

    # Arquivo que não é DICOM e DICOM truncado
    caminho = os.path.join(SAIDA, "nao_dicom.txt")
    with open(caminho, "w", encoding="utf-8") as f:
        f.write("isto nao e um exame\n")
    arquivos.append(caminho)
    with open(os.path.join(SAIDA, "ct_sem_compressao.dcm"), "rb") as f:
        inteiro = f.read()
    caminho = os.path.join(SAIDA, "truncado.dcm")
    with open(caminho, "wb") as f:
        f.write(inteiro[: len(inteiro) - 4000])
    arquivos.append(caminho)

    # .zip com subpasta, como o exportado por um PACS
    with zipfile.ZipFile(os.path.join(SAIDA, "exame.zip"), "w", zipfile.ZIP_DEFLATED) as z:
        for caminho in arquivos:
            z.write(caminho, f"EXAME/{os.path.basename(caminho)}")


if __name__ == "__main__":
    main()
