# UMEP-processing a7798db: lo stato fra un passo e l'altro (estratti)

Estratti verbatim, con il numero di riga, da `UMEP-dev/UMEP-processing` al commit `a7798db1698c29fa5ba667e082e7ad564c20e8e8`
(`main` il 2026-11-15). Licenza del repository: GPL-3.0. Catturati per
`docs/ricerca/04-stato-termico.md`. Il testo del codice non è modificato.

File interi scaricati da `raw.githubusercontent.com` a quel commit:

- `functions/SOLWEIGpython/Solweig_run.py`, sha256 `ef45dfc8e02f11759463fbb1ea738adc8a2a2ca4f544c330b50e9732778ff7d1`
- `functions/SOLWEIGpython/Solweig_2025a_calc_forprocessing.py`, sha256 `046c919066e5723ed37f86d5995c2993c29295615c0d87bcd7a1f2f154251ae9`

## Solweig_run.py:808-828 — passo e stato iniziale

```python
  808      # Initialisation of time related variables
  809      if Ta.__len__() == 1:
  810          timestepdec = 0
  811      else:
  812          timestepdec = dectime[1] - dectime[0]
  813      timeadd = 0.0
  814      # timeaddE = 0.
  815      # timeaddS = 0.
  816      # timeaddW = 0.
  817      # timeaddN = 0.
  818      firstdaytime = 1.0
  819  
  820      # Save hemispheric image
  821      if anisotropic_sky == 1:
  822          if not poisxy is None:
  823              patch_characteristics = hemispheric_image(
  824                  poisxy, shmat, vegshmat, vbshvegshmat, voxelMaps, wallScheme
  825              )
  826  
  827      # If metfile starts at night
  828      CI = 1.0
```

## Solweig_run.py:861-881 — indice di serenità notturno, ricalcolato a mezzanotte

```python
  861          # Nocturnal cloudfraction from Offerle et al. 2003
  862          if (dectime[i] - np.floor(dectime[i])) == 0:
  863              daylines = np.where(np.floor(dectime) == dectime[i])
  864              if daylines.__len__() > 1:
  865                  alt = altitude[0][daylines]
  866                  alt2 = np.where(alt > 1)
  867                  rise = alt2[0][0]
  868                  [_, CI, _, _, _] = clearnessindex_2013b(
  869                      zen[0, i + rise + 1],
  870                      jday[0, i + rise + 1],
  871                      Ta[i + rise + 1],
  872                      RH[i + rise + 1] / 100.0,
  873                      radG[i + rise + 1],
  874                      location,
  875                      P[i + rise + 1],
  876                  )
  877                  if (CI > 1.0) or (CI == np.inf):
  878                      CI = 1.0
  879              else:
  880                  CI = 1.0
  881  
```

## Solweig_2025a_calc_forprocessing.py:220-227 — indice di serenità di giorno

```python
  220      if altitude > 0:  # DAYTIME # # # # # #
  221          # Clearness Index on Earth's surface after Crawford and Dunchon (1999) with a correction
  222          #  factor for low sun elevations after Lindberg et al.(2008)
  223          I0, CI, Kt, I0et, CIuncorr = clearnessindex_2013b(
  224              zen, jday, Ta, RH / 100.0, radG, location, P
  225          )
  226          if (CI > 1) or (CI == np.inf):
  227              CI = 1
```

## Solweig_2025a_calc_forprocessing.py:389-411 — ritardo termico, chiamato solo nel ramo di giorno

```python
  389          # # # # Lup, daytime # # # #
  390          # Surface temperature wave delay - new as from 2014a
  391          Lup, timeaddnotused, Tgmap1 = TsWaveDelay_2015a(
  392              gvfLup, firstdaytime, timeadd, timestepdec, Tgmap1
  393          )
  394          LupE, timeaddnotused, Tgmap1E = TsWaveDelay_2015a(
  395              gvfLupE, firstdaytime, timeadd, timestepdec, Tgmap1E
  396          )
  397          LupS, timeaddnotused, Tgmap1S = TsWaveDelay_2015a(
  398              gvfLupS, firstdaytime, timeadd, timestepdec, Tgmap1S
  399          )
  400          LupW, timeaddnotused, Tgmap1W = TsWaveDelay_2015a(
  401              gvfLupW, firstdaytime, timeadd, timestepdec, Tgmap1W
  402          )
  403          LupN, timeaddnotused, Tgmap1N = TsWaveDelay_2015a(
  404              gvfLupN, firstdaytime, timeadd, timestepdec, Tgmap1N
  405          )
  406  
  407          # # For Tg output in POIs
  408          TgTemp = Tg * shadow + Ta
  409          TgOut, timeadd, TgOut1 = TsWaveDelay_2015a(
  410              TgTemp, firstdaytime, timeadd, timestepdec, TgOut1
  411          )  # timeadd only here v2021a
```

## Solweig_2025a_calc_forprocessing.py:478-522 — fine del ramo di giorno e ramo di notte

```python
  478          )
  479  
  480          firstdaytime = 0
  481  
  482      else:  # # # # # # # NIGHTTIME # # # # # # # #
  483  
  484          Tgwall = 0
  485          # CI_Tg = -999  # F_sh = []
  486  
  487          # Nocturnal K fluxes set to 0
  488          Knight = np.zeros((rows, cols))
  489          Kdown = np.zeros((rows, cols))
  490          Kwest = np.zeros((rows, cols))
  491          Kup = np.zeros((rows, cols))
  492          Keast = np.zeros((rows, cols))
  493          Ksouth = np.zeros((rows, cols))
  494          Knorth = np.zeros((rows, cols))
  495          KsideI = np.zeros((rows, cols))
  496          KsideD = np.zeros((rows, cols))
  497          F_sh = np.zeros((rows, cols))
  498          Tg = np.zeros((rows, cols))
  499          shadow = np.zeros((rows, cols))
  500          CI_Tg = deepcopy(CI)
  501          CI_TgG = deepcopy(CI)
  502          dRad = np.zeros((rows, cols))
  503          Kside = np.zeros((rows, cols))
  504  
  505          # # # # Lup # # # #
  506          Lup = SBC * emis_grid * ((Knight + Ta + Tg + 273.15) ** 4)
  507          if landcover == 1:
  508              Lup[lc_grid == 3] = (
  509                  SBC * 0.98 * (Twater + 273.15) ** 4
  510              )  # nocturnal Water temp
  511  
  512          LupE = Lup
  513          LupS = Lup
  514          LupW = Lup
  515          LupN = Lup
  516  
  517          # # For Tg output in POIs
  518          TgOut = Ta + Tg
  519  
  520          I0 = 0
  521          timeadd = 0
  522          firstdaytime = 1
```
