#!/usr/bin/env python3
"""Estrae da UMEP-dev/solweig il percorso CPU delle ombre, senza Python.

Il vendoring non è una copia fatta a mano: è questo script più il commit
dichiarato in PROVENIENZA.toml. Chi vuole aggiornare al commit successivo non
deve indovinare cosa era stato tolto — riesegue e confronta.

Cosa toglie, e perché:

- `use numpy::…` e `use pyo3::…`. Il nucleo di calcolo prende `ArrayView2` e
  restituisce una struct Rust: Python compare solo negli involucri.
- Ogni elemento annotato `#[pyfunction]`, `#[pyclass]` o `#[pymethods]`. Sono
  gli involucri e la struct specchio del risultato per Python.
- Gli elementi elencati in `DA_TOGLIERE`, che sono involucri senza esserlo per
  attributo: una funzione di conversione verso numpy, o un `impl` di un tipo
  che è appena stato tolto. Nessuna annotazione li marca, quindi vanno nominati.

E una cosa la cambia: `PyResult<T>` diventa `Result<T, String>`, e ogni
`pyo3::exceptions::Py…Error::new_err` diventa `String::from`. È la
trasformazione dichiarata che apre `skyview.rs`, dove il percorso Rust porta
`PyResult` nelle firme perché `create_patches` rifiuta un'opzione di patch
sconosciuta e perché il calcolo si può annullare fra un patch e l'altro.

Sostituire il costruttore invece della costruzione intera è una scelta: così
l'argomento resta identico, parentesi comprese, e la stessa riga di regola vale
per un letterale e per un `format!`, dentro e fuori dal percorso GPU. Il
conteggio atteso è dichiarato per file: se a monte cambia, lo script si ferma
invece di sostituire meno del previsto.

**La trappola.** Quegli elementi sono spesso preceduti da `#[cfg(feature =
"gpu")]` o da doc-comment. Togliendo solo la funzione, l'attributo resta
orfano e si attacca all'elemento seguente: nel caso reale questo cancellava in
silenzio `ShadowingResultRust`, e il compilatore riportava "cannot find type",
non "hai lasciato un attributo per aria". Per questo lo script risale sugli
attributi e sui commenti già emessi prima di rimuovere.

Il percorso GPU resta nel sorgente ma è dietro `#[cfg(feature = "gpu")]`, e la
feature non è dichiarata nel manifesto vendorato: si compila fuori da sé.

Uso:
    python3 vendor/solweig/estrai.py <clone-di-solweig> [destinazione]
    python3 vendor/solweig/estrai.py --prova
"""

import io
import re
import sys
from pathlib import Path

# I file di monte che OMBRA riusa: la chiusura delle dipendenze del passo fuso
# `compute_timestep_rust`, cioe' i moduli di `rust/src/lib.rs` meno `gpu`,
# `morphology`, `pet` e `pipeline_bundles`. Ogni file ha una voce in ogni lista
# qui sotto, anche vuota: un file senza voce fa fermare lo script (KeyError)
# invece di entrare senza che nessuno abbia deciso cosa togliergli.
FILE = [
    "api.rs",
    "emissivity_models.rs",
    "ground.rs",
    "ground_surface.rs",
    "gvf.rs",
    "gvf_geometry.rs",
    "patch_radiation.rs",
    "perez.rs",
    "pipeline.rs",
    "pipeline_inputs.rs",
    "pipeline_radiation.rs",
    "pipeline_scalars.rs",
    "shadowing.rs",
    "sky.rs",
    "skyview.rs",
    "sun.rs",
    "sunlit_shaded_patches.rs",
    "tmrt.rs",
    "utci.rs",
    "vegetation.rs",
    "wall_aspect.rs",
]

# Elementi che a monte sono privati e che qui diventano `pub`. La promozione va
# dichiarata in PROVENIENZA.toml, e si fa qui perche' una patch a mano si
# perderebbe alla prossima estrazione senza che nessuno se ne accorga.
#
# `calculate_shadows_rust`, `ShadowingResultRust` e `calculate_svf_inner` a
# monte sono gia' `pub` e non stanno qui: da quando, lo dice PROVENIENZA.toml.
DA_ESPORRE = {
    "api.rs": [],
    "emissivity_models.rs": [],
    "ground.rs": [],
    "ground_surface.rs": [],
    "gvf.rs": [],
    "gvf_geometry.rs": [],
    "patch_radiation.rs": [],
    "perez.rs": [],
    "pipeline.rs": [],
    "pipeline_inputs.rs": [],
    "pipeline_radiation.rs": [],
    "pipeline_scalars.rs": [],
    "shadowing.rs": [],
    "sky.rs": [],
    "sun.rs": [],
    "sunlit_shaded_patches.rs": [],
    "tmrt.rs": [],
    "utci.rs": [],
    "vegetation.rs": [],
    # A monte e' `pub(crate)` e il modulo e' privato: il passo e il banco la
    # chiamano da fuori. `esponi` sostituisce la visibilita' invece di anteporla.
    "wall_aspect.rs": ["pub(crate) fn compute_wall_aspect_pure("],
    # A monte resta privato: lo chiama soltanto un involucro dello stesso file,
    # che l'estrazione toglie. Senza promozione il compilatore lo segnalerebbe
    # come codice morto, e tagliarlo sarebbe una modifica al codice rimasto.
    "skyview.rs": [
        "fn crop_svf_intermediate",
    ],
}

# Elementi annotati `#[pyfunction]` che non sono involucri: il corpo e' fisica,
# e Python compare solo nell'attributo. Di questi si toglie l'attributo e si
# tiene il resto, che e' la trasformazione piu' conservativa possibile — una
# riga in meno, zero righe cambiate.
#
# Perche' non riscriverli da noi: il corpo di `utci_single` e' la conversione
# documentata da umidita' relativa a pressione di vapore piu' la chiamata al
# polinomio. Copiarlo in OMBRA significherebbe tenere fisica vendorata fuori
# da `vendor/`, dove nessuno la riconoscerebbe come tale al prossimo
# aggiornamento.
DA_TENERE = {
    "api.rs": [],
    "emissivity_models.rs": [],
    "ground.rs": [],
    "ground_surface.rs": [],
    "gvf.rs": [],
    "gvf_geometry.rs": [],
    "patch_radiation.rs": [],
    "perez.rs": [],
    "pipeline.rs": [],
    "pipeline_inputs.rs": [],
    "pipeline_radiation.rs": [],
    # Gli scalari del passo: i campi sono Rust, `#[pyclass]` e `#[pymethods]`
    # sono il solo Python. Perdono l'attributo e i blocchi di metodi.
    "pipeline_scalars.rs": [
        "pub struct WeatherScalars",
        "pub struct HumanScalars",
        "pub struct ConfigScalars",
    ],
    "shadowing.rs": [],
    "sky.rs": [],
    "skyview.rs": [],
    "sun.rs": [],
    "sunlit_shaded_patches.rs": [],
    "tmrt.rs": [],
    "utci.rs": ["pub fn utci_single"],
    "vegetation.rs": [],
    "wall_aspect.rs": [],
}

# Involucri che nessun attributo marca, e che quindi vanno nominati: una
# funzione che converte il risultato in array numpy, e l'`impl Default` di una
# struct che l'estrazione ha appena tolto. Lasciarli dentro non e' un'opzione —
# il primo nomina Python, il secondo nomina un tipo che non esiste piu'.
DA_TOGLIERE = {
    "api.rs": [],
    "emissivity_models.rs": [],
    "ground.rs": [],
    "ground_surface.rs": [],
    "gvf.rs": [],
    "gvf_geometry.rs": [],
    "patch_radiation.rs": [],
    "perez.rs": [],
    # Il ponte che rilascia il GIL, usato solo da `compute_timestep`, che e' un
    # involucro Python e sparisce con il suo attributo.
    "pipeline.rs": ["unsafe fn allow_threads_unchecked"],
    "pipeline_inputs.rs": [],
    "pipeline_radiation.rs": [],
    "pipeline_scalars.rs": [],
    "shadowing.rs": [],
    "sky.rs": [],
    "skyview.rs": [
        "fn svf_intermediate_to_py",
        "impl Default for SkyviewRunner",
    ],
    "sun.rs": [],
    "sunlit_shaded_patches.rs": [],
    "tmrt.rs": [],
    "utci.rs": [],
    "vegetation.rs": [],
    "wall_aspect.rs": ["impl Default for WallAspectRunner"],
}

# Quante firme `PyResult<T>` e quante costruzioni di errore ci si aspetta di
# trovare in ciascun file, dopo le rimozioni. Il numero e' dichiarato invece che
# contato perche' e' il numero a fermare lo script: se a monte una firma
# compare o sparisce, la sostituzione non e' piu' quella descritta in
# PROVENIENZA.toml e va riletta prima di fidarsene.
SOSTITUZIONI = {
    "api.rs": (0, 0),
    "emissivity_models.rs": (0, 0),
    "ground.rs": (0, 0),
    "ground_surface.rs": (0, 0),
    "gvf.rs": (0, 0),
    "gvf_geometry.rs": (0, 0),
    "patch_radiation.rs": (0, 0),
    "perez.rs": (0, 0),
    "pipeline.rs": (0, 0),
    "pipeline_inputs.rs": (0, 0),
    "pipeline_radiation.rs": (0, 0),
    "pipeline_scalars.rs": (0, 0),
    "shadowing.rs": (0, 0),
    "sky.rs": (0, 0),
    "skyview.rs": (3, 11),
    "sun.rs": (0, 0),
    "sunlit_shaded_patches.rs": (0, 0),
    "tmrt.rs": (0, 0),
    "utci.rs": (0, 0),
    "vegetation.rs": (0, 0),
    "wall_aspect.rs": (0, 0),
}

# Le regole che riscrivono il testo senza passare dagli elenchi sopra, con
# quante volte ci si aspetta che scattino in ciascun file. Un file che non
# compare ne aspetta zero. Come `SOSTITUZIONI`, e' il numero a fermare lo script.
CONTEGGI = {
    "use_multiriga": {"sky.rs": 1},
    "attributi_di_campo": {"pipeline_scalars.rs": 34},
    "pub_use_bundles": {"pipeline.rs": 1},
}


def dichiarazione(righe: list[str], i: int) -> str:
    """La dichiarazione dell'elemento che comincia a `i`, saltando attributi e commenti."""
    while i < len(righe):
        spoglia = righe[i].strip()
        if spoglia.startswith("#[") or spoglia.startswith("///") or not spoglia:
            i += 1
            continue
        return " ".join(spoglia.split()[:3]).rstrip("(")
    return ""


def arretra(fuori: list[str]) -> None:
    """Toglie da `fuori` gli attributi e i commenti dell'elemento che sta per sparire.

    Un `#[cfg]` lasciato orfano si attacca all'elemento seguente e lo cancella
    in silenzio; un commento lasciato orfano descrive codice che non c'è più,
    che è la specie di bugia che questo progetto insegue da tre volte.
    """
    while fuori and (
        fuori[-1].lstrip().startswith("#[")
        or fuori[-1].lstrip().startswith("//")
        or fuori[-1].strip() == ""
    ):
        fuori.pop()


def estrai(testo: str, da_tenere: list[str], da_togliere: list[str]) -> tuple[str, int, int]:
    """Il sorgente senza gli involucri Python, quanti ne ha tolti e quanti ne ha tenuti."""
    righe = testo.split("\n")
    fuori: list[str] = []
    i = tolti = tenuti = 0
    while i < len(righe):
        riga = righe[i]
        if riga.startswith("use numpy::") or riga.startswith("use pyo3::"):
            i += 1
            continue
        per_attributo = riga.strip() in ("#[pyfunction]", "#[pyclass]", "#[pymethods]")
        per_nome = any(riga.startswith(e) for e in da_togliere)
        if per_attributo or per_nome:
            # Un elemento dichiarato in DA_TENERE perde l'attributo e resta.
            if per_attributo and any(dichiarazione(righe, i + 1).startswith(e) for e in da_tenere):
                tenuti += 1
                i += 1
                continue
            arretra(fuori)
            # In codice formattato un elemento di primo livello chiude con una
            # graffa in colonna zero.
            i += 1
            while i < len(righe) and righe[i] != "}":
                i += 1
            i += 1
            tolti += 1
            continue
        fuori.append(riga)
        i += 1
    return "\n".join(fuori), tolti, tenuti


# Il costruttore di errore di pyo3, in tutte le sue specie: `PyValueError`,
# `PyRuntimeError`, `PyInterruptedError`. Diventa il costruttore di `String`,
# che è la sostituzione più meccanica disponibile — l'argomento resta identico,
# parentesi comprese, e non serve bilanciarle per riscriverlo.
COSTRUTTORE = re.compile(r"pyo3::exceptions::Py\w+::new_err")


def firma_senza_pyresult(riga: str) -> str:
    """`-> PyResult<T>` diventa `-> Result<T, String>`, con T anche annidato."""
    inizio = riga.index("-> PyResult<")
    apertura = inizio + len("-> PyResult<")
    profondita, j = 1, apertura
    while j < len(riga) and profondita:
        if riga[j] == "<":
            profondita += 1
        elif riga[j] == ">":
            profondita -= 1
        j += 1
    if profondita:
        raise ValueError(f"parametro di PyResult non chiuso: {riga!r}")
    return f"{riga[:inizio]}-> Result<{riga[apertura:j - 1]}, String>{riga[j:]}"


def senza_pyresult(testo: str) -> tuple[str, int, int]:
    """Il sorgente con `Result<T, String>` al posto di `PyResult<T>`.

    Restituisce anche quante firme e quante costruzioni di errore ha toccato,
    perché è il conteggio a dire se a monte la sostituzione è ancora quella.
    """
    righe = testo.split("\n")
    firme = errori = 0
    for k, riga in enumerate(righe):
        if "-> PyResult<" in riga:
            riga = firma_senza_pyresult(riga)
            firme += 1
        riga, quanti = COSTRUTTORE.subn("String::from", riga)
        errori += quanti
        righe[k] = riga
    return "\n".join(righe), firme, errori


def esponi(testo: str, da_esporre: list[str]) -> tuple[str, int]:
    """Promuove a `pub` gli elementi privati indicati. Restituisce quanti ne ha promossi.

    La riga di un elemento che a monte ha già una visibilità comincia con `pub`,
    non con la dichiarazione cercata, e non viene contata: il conteggio in
    `main` allora non torna, e lo script si ferma invece di lasciar credere di
    averlo promosso.
    """
    righe = testo.split("\n")
    fatti = 0
    for k, riga in enumerate(righe):
        for elemento in da_esporre:
            if riga.startswith(elemento):
                # `pub(crate) fn` diventa `pub fn`: anteporre darebbe `pub pub(crate)`.
                if elemento.startswith("pub(crate) "):
                    righe[k] = riga.replace("pub(crate) ", "pub ", 1)
                else:
                    righe[k] = f"pub {riga}"
                fatti += 1
    return "\n".join(righe), fatti


def senza_use_multiriga(testo: str) -> tuple[str, int]:
    """Toglie un `use numpy::{…};` o `use pyo3::{…};` che va a capo, fino al `;`.

    `estrai` toglie la riga sola e lascerebbe il resto dell'elenco, che non
    compila. Quello su una riga sola e' compito suo e non si conta qui.
    """
    righe = testo.split("\n")
    fuori: list[str] = []
    i = fatti = 0
    while i < len(righe):
        riga = righe[i]
        if riga.startswith(("use numpy::{", "use pyo3::{")) and not riga.rstrip().endswith(";"):
            while not righe[i].rstrip().endswith(";"):
                i += 1
            i += 1
            fatti += 1
            continue
        fuori.append(riga)
        i += 1
    return "\n".join(fuori), fatti


def senza_attributi_di_campo(testo: str) -> tuple[str, int]:
    """Toglie le righe `#[pyo3(get, set)]` che restano sui campi delle struct tenute."""
    righe = testo.split("\n")
    fuori = [r for r in righe if r.strip() != "#[pyo3(get, set)]"]
    return "\n".join(fuori), len(righe) - len(fuori)


def senza_pub_use_bundles(testo: str) -> tuple[str, int]:
    """Toglie `pub use crate::pipeline_bundles::{…};` e il commento che lo introduce.

    Non e' un elemento di `DA_TOGLIERE`: `estrai` cerca una graffa chiusa in
    colonna zero, e questo elenco chiude con `};`. Lo script non si fermerebbe
    li' e cancellerebbe in silenzio il codice seguente, fino alla prima `}`.
    """
    righe = testo.split("\n")
    fuori: list[str] = []
    i = fatti = 0
    while i < len(righe):
        if righe[i].startswith("pub use crate::pipeline_bundles::{"):
            arretra(fuori)
            while righe[i] != "};":
                i += 1
            i += 1
            fatti += 1
            continue
        fuori.append(righe[i])
        i += 1
    return "\n".join(fuori), fatti


def verifica(nome: str, regola: str, trovati: int, attesi: int) -> int:
    """0 se la regola e' scattata quanto dichiarato, 1 e un messaggio altrimenti."""
    if trovati == attesi:
        return 0
    print(
        f"{nome}: la regola `{regola}` doveva scattare {attesi} volte, e' scattata {trovati}. "
        "A monte il sorgente ha cambiato forma: rileggi CONTEGGI prima di fidarti.",
        file=sys.stderr,
    )
    return 1


# Identificatori di Python nel codice: guardare le sottostringhe scattava su
# commenti e letterali, e non vedeva `PyReadonlyArray2` ne' `Python`.
IDENTIFICATORI_PYTHON = re.compile(r"\b(pyo3|numpy)::|\bPy[A-Z]\w*|\bPython\b|\bBound<|#\[py")


def residui(testo: str) -> list[int]:
    """I numeri di riga (da 1) con codice Python fuori dai commenti `//`.

    Limite noto: taglia anche un `//` dentro un letterale stringa. Il residuo
    comparirebbe invece di sparire, quindi l'errore sarebbe rumoroso.
    """
    return [
        n
        for n, riga in enumerate(testo.split("\n"), 1)
        if IDENTIFICATORI_PYTHON.search(riga.split("//", 1)[0])
    ]


def prova() -> int:
    """Esegue i casi della regola dei residui. 0 se tornano tutti, 1 altrimenti."""
    casi = [
        ("// a numpy view\nlet x = 1;\n", []),
        ('let s = "contiguous numpy views";\n', []),
        ("//! Takes PyArray inputs, as upstream does.\nlet x = 1;\n", []),
        ("use numpy::PyArray2;\n", [1]),
        ("fn f(a: PyReadonlyArray2<f32>) {}\n", [1]),
        ("let p: Python = py;\n", [1]),
        ("fn f(x: Bound<'_, PyAny>) {}\n", [1]),
        ("#[pyfunction]\nfn f() {}\n", [1]),
        ("use pyo3::prelude::*;\n", [1]),
    ]
    sbagliati = 0
    for testo, atteso in casi:
        trovato = residui(testo)
        if trovato != atteso:
            print(f"{testo!r}: attesi {atteso}, trovati {trovato}", file=sys.stderr)
            sbagliati += 1

    # Le regole che riscrivono: sorgente, uscita attesa, quanti elementi toccati.
    regole = [
        (senza_use_multiriga, "use numpy::{\n    PyArray2,\n};\nfn f() {}\n", "fn f() {}\n", 1),
        (
            senza_attributi_di_campo,
            "#[pyo3(get, set)]\n    pub ta: f32,\n",
            "    pub ta: f32,\n",
            1,
        ),
        # Con DA_TOGLIERE la regola cancellerebbe anche `dopo`: `};` non e' `}`.
        (
            senza_pub_use_bundles,
            "pub use crate::pipeline_bundles::{\n    A,\n};\nfn dopo() {\n}\n",
            "fn dopo() {\n}\n",
            1,
        ),
        (
            lambda t: esponi(t, ["pub(crate) fn compute_wall_aspect_pure("]),
            "pub(crate) fn compute_wall_aspect_pure(\n",
            "pub fn compute_wall_aspect_pure(\n",
            1,
        ),
    ]
    for regola, testo, atteso, quanti in regole:
        trovato = regola(testo)
        if trovato != (atteso, quanti):
            print(f"{testo!r}: attesi {(atteso, quanti)!r}, trovati {trovato!r}", file=sys.stderr)
            sbagliati += 1

    # Una regola con conteggio dichiarato 2 su un sorgente che ne contiene 1.
    _, trovati = senza_use_multiriga("use numpy::{\n    A,\n};\n")
    stderr, sys.stderr = sys.stderr, io.StringIO()
    try:
        esito = verifica("x.rs", "use_multiriga", trovati, 2)
        messaggio = sys.stderr.getvalue()
    finally:
        sys.stderr = stderr
    if esito != 1 or "use_multiriga" not in messaggio:
        print(f"conteggio che non torna: esito {esito}, messaggio {messaggio!r}", file=sys.stderr)
        sbagliati += 1
    return int(sbagliati > 0)


def main() -> int:
    if sys.argv[1:] == ["--prova"]:
        return prova()
    if len(sys.argv) < 2:
        print(__doc__.strip().split("Uso:")[1].strip(), file=sys.stderr)
        return 2

    sorgente = Path(sys.argv[1]) / "rust" / "src"
    destinazione = Path(sys.argv[2]) if len(sys.argv) > 2 else Path(__file__).parent / "src"
    if not sorgente.is_dir():
        print(f"{sorgente}: non è la cartella rust/src di un clone di solweig", file=sys.stderr)
        return 1
    destinazione.mkdir(parents=True, exist_ok=True)

    for nome in FILE:
        da_esporre = DA_ESPORRE[nome]
        da_tenere = DA_TENERE[nome]
        testo = (sorgente / nome).read_text(encoding="utf-8")
        pulito, multiriga = senza_use_multiriga(testo)
        pulito, tolti, tenuti = estrai(pulito, da_tenere, DA_TOGLIERE[nome])
        pulito, campi = senza_attributi_di_campo(pulito)
        pulito, bundles = senza_pub_use_bundles(pulito)
        pulito, firme, errori = senza_pyresult(pulito)
        pulito, esposti = esponi(pulito, da_esporre)
        righe_residue = residui(pulito)
        if righe_residue:
            print(
                f"{nome}: restano {len(righe_residue)} righe che nominano Python "
                f"(da 1): {righe_residue}",
                file=sys.stderr,
            )
            return 1
        (destinazione / nome).write_text(pulito, encoding="utf-8")
        prima = len(testo.split("\n"))
        dopo = len(pulito.split("\n"))
        print(
            f"{nome}: {prima} -> {dopo} righe, {tolti} involucri rimossi, "
            f"{tenuti} tenuti senza attributo, {esposti} elementi esposti, "
            f"{firme} firme e {errori} errori senza PyResult, {multiriga} use a capo, "
            f"{campi} attributi di campo, {bundles} pub use dei bundle"
        )
        for regola, trovati in (
            ("use_multiriga", multiriga),
            ("attributi_di_campo", campi),
            ("pub_use_bundles", bundles),
        ):
            if verifica(nome, regola, trovati, CONTEGGI[regola].get(nome, 0)):
                return 1
        if (firme, errori) != SOSTITUZIONI[nome]:
            attese, attesi = SOSTITUZIONI[nome]
            print(
                f"{nome}: attese {attese} firme e {attesi} costruzioni di errore, "
                f"trovate {firme} e {errori}. A monte il percorso Rust ha cambiato "
                "forma: rileggi SOSTITUZIONI prima di fidarti.",
                file=sys.stderr,
            )
            return 1
        if esposti != len(da_esporre):
            print(
                f"{nome}: attesi {len(da_esporre)} elementi da esporre, promossi {esposti}. "
                "A monte le visibilita' sono cambiate: rileggi DA_ESPORRE prima di fidarti.",
                file=sys.stderr,
            )
            return 1
        if tenuti != len(da_tenere):
            print(
                f"{nome}: attesi {len(da_tenere)} elementi da tenere, tenuti {tenuti}. "
                "A monte quegli elementi non ci sono piu' o non sono piu' annotati: "
                "rileggi DA_TENERE prima di fidarti.",
                file=sys.stderr,
            )
            return 1

    # Anche `lib.rs` esce da qui, e non da una mano: cosi' l'intera cartella
    # `src` e' prodotto dell'estrazione, e il controllo in integrazione continua
    # puo' confrontarla per intero invece di sapere quali file ignorare.
    moduli = "\n".join(f"pub mod {nome.removesuffix('.rs')};" for nome in FILE)
    (destinazione / "lib.rs").write_text(
        "// Vendorato da UMEP-dev/solweig, GPL-3. Vedi PROVENIENZA.toml.\n"
        "// Generato da estrai.py: non si modifica a mano.\n"
        f"\n{moduli}\n",
        encoding="utf-8",
    )

    return 0


if __name__ == "__main__":
    sys.exit(main())
