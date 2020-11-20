use anyhow::{anyhow, Context, Result};
use clap::Clap;
use riff_ani::ico::{IconDir, IconDirEntry, IconImage, ResourceType};
use riff_ani::{Ani, AniHeader};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

/// This program reads the config file to find the list of cursor x2 in PNG format along with
/// their hotspot and nominal size, then converts all of the x2 to CUR or ANI format.
///
/// The config file format is the same with xcursorgen. Each line in the config is of the form:
///
///     <size> <x-hot> <y-hot> <filename> <ms-delay>
///
/// Multiple images with the same <size> are used to create animated cursors, the <ms-delay> value
/// on each line indicates how long each image should be displayed before switching to the next.
/// <ms-delay> can be elided for static cursors.
///
/// Note: on Windows, the frame rate of animated cursor is in terms of jiffies (1/60 sec), so the
/// difference of <ms-delay> will not take effect precisely. For example, both `30 ms` and `40 ms`
/// result in `round(30 / 16.667) = round(40 / 16.667) = 2 jiffies` in the generated cursor file.
#[derive(Clap, Debug)]
#[clap(version = "0.1", author = "Balthild <ibalthild@gmail.com>")]
struct Opts {
    /// The path of config file
    #[clap(short, long)]
    config: PathBuf,
    /// Find cursor x2 in the directory. If not specified, the current directory is used.
    #[clap(short, long)]
    prefix: Option<PathBuf>,
    /// The path of output file without file ext (a .cur or .ani ext will be
    /// automatically appended according to whether the cursor is animated)
    #[clap(short, long)]
    output: PathBuf,
    /// Choose which size to generate. Unlike xcursor, one ANI file cannot contain multiple x2
    /// in different sizes, so we must pick up one. The size specified must exist in the config.
    #[clap(short, long)]
    size: u16,
}

#[derive(Debug)]
struct FrameConfig {
    size: u16,
    x_hot: u16,
    y_hot: u16,
    path: PathBuf,
    ms_delay: u32,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    if opts.output.file_name().is_none() {
        return Err(anyhow!("invalid output path"));
    }

    let config = parse_config(&opts.config)?;
    match config.get(&opts.size) {
        None => Err(anyhow!("the size does not exist in the config")),
        Some(frames) => match frames.as_slice() {
            [] => unreachable!(),
            [x] => generate_cur(x, &opts),
            xs => generate_ani(xs, &opts),
        },
    }
}

fn parse_config(path: &Path) -> Result<HashMap<u16, Vec<FrameConfig>>> {
    let mut data = String::new();
    File::open(path)
        .context("cannot open config file")?
        .read_to_string(&mut data)
        .context("cannot read config file")?;

    let mut result = HashMap::new();
    for (i, frame) in data.lines().map(parse_config_line).enumerate() {
        let frame = frame.map_err(|e| {
            let p = path.to_string_lossy();
            anyhow!("invalid config file. {}\nat line {} of {}", e, i, p)
        })?;

        let certain_size = result.entry(frame.size).or_insert_with(Vec::new);
        certain_size.push(frame);
    }

    Ok(result)
}

fn parse_config_line(line: &str) -> Result<FrameConfig, &'static str> {
    let cols: Vec<_> = line.split_ascii_whitespace().collect();
    match cols.len() {
        4 | 5 => Ok(FrameConfig {
            size: cols[0].parse().map_err(|_| "<size> must be an integer")?,
            x_hot: cols[1].parse().map_err(|_| "<x-hot> must be an integer")?,
            y_hot: cols[2].parse().map_err(|_| "<y-hot> must be an integer")?,
            path: cols[3].into(),
            ms_delay: {
                let value = cols.get(4).cloned().unwrap_or("0");
                value.parse().map_err(|_| "<ms-delay> must be an integer")?
            },
        }),
        _ => Err("Unrecognizable format"),
    }
}

fn generate_cur(frame: &FrameConfig, opts: &Opts) -> Result<()> {
    let mut filename = opts.output.file_name().unwrap().to_os_string();
    filename.push(".cur");

    let cur = create_cur(frame, opts)?;

    let dest = opts.output.with_file_name(filename);
    let out = File::create(&dest).with_context(|| {
        let p = dest.to_string_lossy();
        format!("cannot create cursor file {}", p)
    })?;

    cur.write(&out).with_context(|| {
        let p = dest.to_string_lossy();
        format!("cannot write cursor file {}", p)
    })
}

fn generate_ani(frames: &[FrameConfig], opts: &Opts) -> Result<()> {
    if frames.iter().any(|x| x.ms_delay == 0) {
        return Err(anyhow!(
            "the <ms-delay> must be specified for animated cursor"
        ));
    }

    let mut filename = opts.output.file_name().unwrap().to_os_string();
    filename.push(".ani");

    let ani = Ani {
        header: AniHeader {
            num_frames: frames.len() as u32,
            num_steps: frames.len() as u32,
            width: opts.size as u32,
            height: opts.size as u32,
            frame_rate: (frames[0].ms_delay as f32 / 16.667).round() as u32,
        },
        frames: frames
            .iter()
            .map(|x| create_cur(x, opts))
            .collect::<Result<_>>()?,
    };

    let dest = opts.output.with_file_name(filename);
    let out = File::create(&dest).with_context(|| {
        let p = dest.to_string_lossy();
        format!("cannot create cursor file {}", p)
    })?;

    ani.encode(&out).with_context(|| {
        let p = dest.to_string_lossy();
        format!("cannot write cursor file {}", p)
    })?;

    Ok(())
}

fn create_cur(frame: &FrameConfig, opts: &Opts) -> Result<IconDir> {
    let path = match &opts.prefix {
        Some(prefix) => prefix.join(&frame.path),
        None => frame.path.clone(),
    };

    let file = std::fs::File::open(&path).with_context(|| {
        let p = path.to_string_lossy();
        format!("cannot open PNG file {}", p)
    })?;

    let mut image = IconImage::read_png(file).with_context(|| {
        let p = path.to_string_lossy();
        format!("cannot read PNG file {}", p)
    })?;
    image.set_cursor_hotspot(Some((frame.x_hot, frame.y_hot)));

    let entry = IconDirEntry::encode_as_png(&image).context("cannot encode PNG to CUR/ANI")?;
    let mut icon_dir = IconDir::new(ResourceType::Cursor);
    icon_dir.add_entry(entry);

    Ok(icon_dir)
}
