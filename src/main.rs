use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use anyhow::Result;
use reqwest;
use scraper::{selectable::Selectable, Html, Selector};
use tokio::sync::mpsc;

#[derive(Debug)]
struct ParliamentaryLink {
    name: String,
    link: String,
}

#[derive(Debug)]
struct FinalParliametaryData {
    name: String,
    date_of_birth: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut deputies_list: VecDeque<ParliamentaryLink> = VecDeque::new();
    let mut page: u8 = 1;
    let mut still_get_link: bool = true;
    let threads_amount: usize = 12;

    while still_get_link == true {
        let format_url = format!(
            "https://www.camara.leg.br/deputados/quem-sao/resultado?search=&partido=&uf=&legislatura=&sexo=&pagina={}",
            page
        );
        println!("scraping page -> {:?}", format_url);
        let scrapad_page = scrape_list_page(&format_url).await;

        if scrapad_page.len() == 0 {
            still_get_link = false;
        }

        deputies_list.extend(scrapad_page);
        page += 1;
    }

    let initial_current_size_list = deputies_list.len();
    let bench_amount: usize = initial_current_size_list / threads_amount;
    let mut benches: HashMap<u8, Vec<ParliamentaryLink>> = HashMap::new();

    for i in 0..threads_amount {
        let current_position = i + 1;
        let mut temp: Vec<ParliamentaryLink> = Vec::new();

        if current_position == threads_amount {
            let final_size =
                (initial_current_size_list - (bench_amount * threads_amount)) + bench_amount;

            for z in 0..final_size {
                let item = deputies_list.get(z).unwrap();

                temp.push(ParliamentaryLink {
                    link: item.link.to_owned(),
                    name: item.name.to_owned(),
                });
            }

            for _k in 0..final_size {
                deputies_list.pop_front();
            }

            benches.insert(i.try_into().unwrap(), temp);
            continue;
        }

        for x in 0..bench_amount {
            let item = deputies_list.get(x).unwrap();
            temp.push(ParliamentaryLink {
                link: item.link.to_owned(),
                name: item.name.to_owned(),
            });
        }

        for _l in 0..bench_amount {
            deputies_list.pop_front();
        }

        benches.insert(i.try_into().unwrap(), temp);
    }

    let (tx, mut rx) = mpsc::channel::<FinalParliametaryData>(100);
    let secure_share = Arc::new(benches);

    for th in 0..threads_amount {
        let tx_clone = tx.clone();
        let manipulator = Arc::clone(&secure_share);

        tokio::spawn(async move {
            let current_index: u8 = th.try_into().unwrap();
            let bunch_selected = manipulator.get(&current_index).unwrap();

            for i in 0..bunch_selected.len() {
                let select_item = bunch_selected.get(i).unwrap();
                let date_of_birth = date_of_birth(&select_item.link).await;

                tx_clone
                    .send(FinalParliametaryData {
                        date_of_birth,
                        name: select_item.name.to_owned(),
                    })
                    .await
                    .unwrap();
            }
        });
    }

    drop(tx);
    while let Some(data) = rx.recv().await {
        println!(
            "nome {} - data de nascimento: {}",
            data.name, data.date_of_birth
        );
    }

    Ok(())
}

async fn date_of_birth(url: &str) -> String {
    let response_html = reqwest::get(url).await.unwrap().text().await.unwrap();
    let document = Html::parse_document(&response_html);
    let mut date = "";

    let selector = Selector::parse("ul.informacoes-deputado").unwrap();
    let li_selector = Selector::parse("li").unwrap();

    let lis = document.select(&selector).next();

    if lis.is_none() {
        return String::from("");
    }

    for item in lis.unwrap().select(&li_selector) {
        let item_content = item.text().collect::<Vec<&str>>();

        if item_content.get(0).unwrap().contains("Data") {
            date = item_content.get(1).unwrap().trim();
        }
    }

    date.to_owned()
}

async fn scrape_list_page(url: &str) -> Vec<ParliamentaryLink> {
    let avaliable_tags: [&str; 2] = ["Em exercício", "Licenciado"];
    let html_document = reqwest::get(url).await.unwrap().text().await.unwrap();
    let mut scraped_data: Vec<ParliamentaryLink> = Vec::new();
    let document = Html::parse_document(&html_document);
    let selector_names = Selector::parse("h3.lista-resultados__cabecalho").unwrap();

    document.select(&selector_names).for_each(|element| {
        let selector_link = Selector::parse("a").unwrap();
        let selector_span_tag = Selector::parse("span").unwrap();

        let tag_value = element
            .select(&selector_span_tag)
            .next()
            .unwrap()
            .inner_html();

        if avaliable_tags.contains(&tag_value.as_str()) {
            let a_tag = element.select(&selector_link).next().unwrap();

            scraped_data.push(ParliamentaryLink {
                name: a_tag.inner_html().to_owned(),
                link: a_tag.value().attr("href").unwrap().to_owned(),
            });
        }
    });

    scraped_data
}
