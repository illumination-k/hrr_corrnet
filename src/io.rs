use std::{fmt::{Display, Debug}, path::Path, str::FromStr};
use std::io::{BufRead, BufReader};
use std::collections::HashSet;

use flate2::read::MultiGzDecoder;

use csv::{Reader, Writer};
use anyhow::Result;
use ndarray::{ArrayBase, Array2};

use crate::graph;
use crate::math;

pub fn read_exp_csv<P: AsRef<Path>>(
    input: P, 
    index: &mut Vec<String>, 
) -> Result<Array2<f64>> {
    let mut shape = (0, 0);
    let mut vec: Vec<f64> = vec![];
    let mut rdr = Reader::from_path(input)?;

    for _r in rdr.records() {
        let r = _r?;
        // skip index
        let exp_vec: Vec<f64> = r.into_iter()
            .skip(1)
            .map(|x| x.parse::<f64>().expect("cannot convert to f64"))
            .collect();
        
        // Whine std == 0, continue. Maybe use approx for abs_diff_eq
        if math::std(&exp_vec, 1.) == 0. { continue; }

        index.push(r[0].to_string());
        vec.extend(exp_vec);
        shape.1 = r.len()-1;
        shape.0 += 1;
        
    }
    info!("shape: {:?}", shape);
    info!("calculate pearson correlation...");
    Ok(ArrayBase::from_shape_vec(shape, vec)?)
}

// pub fn read_graph_csv<P, T>(p: P) -> Result<graph::Graph<T>> 
//     where P: AsRef<Path>, T: Clone + Copy + Debug + FromStr + Display, <T as FromStr>::Err: std::fmt::Debug
// {
//     let mut rdr = Reader::from_path(p)?;
//     let mut records: Vec<CsvRecord> = vec![];

//     for _r in rdr.deserialize() {
//         let r: CsvRecord = _r?;
//         records.push(r)
//     }

//     Ok(graph::Graph::from_records(&records))
// }

pub fn read_gene_list<P: AsRef<Path>>(p: &P) -> Result<HashSet<String>> {
    let mut rdr = Reader::from_path(p)?;

    let mut res: HashSet<String> = HashSet::new();

    for _r in rdr.records() {
        let r = _r?;
        res.insert(r.into_iter().nth(0).unwrap().to_string());
    }

    Ok(res)
}

fn open_with_gz<P: AsRef<Path>>(p: P) -> Result<Box<dyn BufRead>> {
    let r = std::fs::File::open(p.as_ref())?;
    let ext = p.as_ref().extension();

    if ext == Some(std::ffi::OsStr::new("gz")) {
        let gz = MultiGzDecoder::new(r)?;
        let buf_reader = BufReader::new(gz);
        Ok(Box::new(buf_reader))
    } else {
        let buf_reader = BufReader::new(r);
        Ok(Box::new(buf_reader))
    }
}

pub fn read_fasta<P: AsRef<Path>>(fasta: P) -> Result<(Vec<String>, Vec<String>)> {
    let mut index = vec![];
    let mut seqs = vec![];
    let rdr = bio::io::fasta::Reader::new(open_with_gz(fasta)?);
    
    for _r in rdr.records() {
        let r = _r?;
        index.push(r.id().to_string());
        seqs.push(String::from_utf8(r.seq().to_vec())?);
    }

    Ok((index, seqs))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CsvRecord {
    gene_1: String,
    gene_2: String,
    corr: f64,
    rank: String,
}

impl CsvRecord {
    pub fn new(gene_1: String, gene_2: String, corr: f64, rank: String) -> Self {
        Self {
            gene_1,
            gene_2,
            corr,
            rank
        }
    }

    pub fn genes(&self) -> (String, String) {
        (self.gene_1.clone(), self.gene_2.clone())
    }

    #[allow(dead_code)]
    pub fn corr(&self) -> f64 {
        self.corr
    }

    pub fn rank<T: FromStr + Debug>(&self) -> T
        where <T as FromStr>::Err: std::fmt::Debug
    {
        self.rank.parse().unwrap()
    }

    pub fn from_tuple(edge: &(String, String, f64, f64)) -> Self {
        Self {
            gene_1: edge.0.to_owned(),
            gene_2: edge.1.to_owned(),
            corr: edge.2,
            rank: edge.3.to_string()
        }
    }

    pub fn from_byte_records(rec: &ByteCsvRecord) -> Self {
        Self {
            gene_1: rec.gene_1_unchecked(),
            gene_2: rec.gene_2_unchecked(),
            corr: rec.corr(),
            rank: rec.rank().to_string()
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ByteCsvRecord<'a> {
    gene_1: &'a [u8],
    gene_2: &'a [u8],
    corr: f64,
    rank: f64,
}

impl ByteCsvRecord<'_> {
    pub fn gene_1_unchecked(&self) -> String {
        unsafe {
            String::from_utf8_unchecked(self.gene_1.to_vec())
        }
    }

    pub fn gene_2_unchecked(&self) -> String {
        unsafe {
            String::from_utf8_unchecked(self.gene_2.to_vec())
        }
    }

    pub fn genes_unchecked(&self) -> (String, String) {
        (self.gene_1_unchecked(), self.gene_2_unchecked())
    }

    pub fn gene_1_bytes(&self) -> &[u8] {
        self.gene_1
    }

    pub fn gene_2_bytes(&self) -> &[u8] {
        self.gene_2
    }

    #[allow(dead_code)]
    pub fn genes_bytes(&self) -> (&[u8], &[u8]) {
        (self.gene_1, self.gene_2)
    }

    #[allow(dead_code)]
    pub fn gene_1(&self) -> std::result::Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.gene_1.to_vec())
    }

    #[allow(dead_code)]
    pub fn gene_2(&self) -> std::result::Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.gene_2.to_vec())
    }

    #[allow(dead_code)]
    pub fn genes(&self) -> Result<(String, String)> {
        Ok((self.gene_1()?, self.gene_2()?))
    }


    pub fn corr(&self) -> f64 {
        self.corr
    }

    pub fn rank(&self) -> f64 {
        self.rank
    }
 }


pub fn graph_to_csv<P, T>(
    outpath: P, 
    graph: graph::Graph<T>
) -> Result<()>
    where P: AsRef<Path>, T: Copy + Clone + Display + PartialOrd + PartialEq + FromStr
{
    let mut wtr = Writer::from_path(outpath.as_ref())?;

    for edge in graph.edges().iter() {
        wtr.serialize(edge.to_record(graph.nodes()))?;
    }

    wtr.flush()?;    

    Ok(())
}