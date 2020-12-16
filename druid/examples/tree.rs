// Copyright 2019 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Demos basic tree widget and tree manipulations.
use std::vec::Vec;

//use druid::im::{vector, Vector};
//use druid::lens::{self, LensExt};
use druid::widget::{LabelText, Tree, TreeNode, Scroll};
use druid::{
    AppLauncher, Data, Lens, LocalizedString, Widget, WindowDesc,
};

#[derive(Clone, Lens)]
struct Taxonomy {
	name: &'static str,
	children: Vec<Taxonomy>,
}

impl Taxonomy {
	fn new(name: &'static str) -> Self {
		Taxonomy { name, children: Vec::new() }
	}

	fn add_child(mut self, child : Self) -> Self {
		self.children.push(child);
		self
	}
}

impl Data for Taxonomy {
	fn same(&self, other: &Self) -> bool {
		self.name.same(&other.name) && self.children.len() == other.children.len() && self.children.iter().zip(other.children.iter()).all(|(a, b)| a.same(b))
	}
}

impl TreeNode for Taxonomy {
	fn label_text(&self) -> LabelText<()> {
		LabelText::from(self.name)
	}

	fn children_count(&self) -> usize {
		self.children.len()
	}
	
	fn get_child(&self, index: usize) -> &Taxonomy {
		&self.children[index]
	}
	
	fn get_child_mut(&mut self, index: usize) -> &mut Taxonomy {
		&mut self.children[index]
	}
}

pub fn main() {
	// Create the main window
    let main_window = WindowDesc::new(ui_builder)
        .title(LocalizedString::new("tree-demo-window-title").with_placeholder("Tree Demo"));

    // Set our initial data. 
	// This is an extract from https://en.wikipedia.org/wiki/Linnaean_taxonomy
    let taxonomy = Taxonomy::new("Life")
		.add_child(Taxonomy::new("Animalia")
			.add_child(Taxonomy::new("Mammalia")
				.add_child(Taxonomy::new("Primates")
					.add_child(Taxonomy::new("Homo")
						.add_child(Taxonomy::new("Homo sapiens"))
						.add_child(Taxonomy::new("Homo troglodytes"))
					)
					.add_child(Taxonomy::new("Simia"))
					.add_child(Taxonomy::new("Lemur")
						.add_child(Taxonomy::new("Lemur tardigradus"))
						.add_child(Taxonomy::new("Lemur catta"))
						.add_child(Taxonomy::new("Lemur volans"))
					)
					.add_child(Taxonomy::new("Vespertilio"))
				)
				.add_child(Taxonomy::new("Bruta"))
				.add_child(Taxonomy::new("Ferae"))
				.add_child(Taxonomy::new("Bestiae"))
				.add_child(Taxonomy::new("Glires"))
				.add_child(Taxonomy::new("Pecora"))
				.add_child(Taxonomy::new("Belluae"))
				.add_child(Taxonomy::new("Cete"))
			)
			.add_child(Taxonomy::new("Aves")
				.add_child(Taxonomy::new("Accipitres"))
				.add_child(Taxonomy::new("Picae"))
				.add_child(Taxonomy::new("Anseres"))
				.add_child(Taxonomy::new("Grallae"))
				.add_child(Taxonomy::new("Gallinae"))
				.add_child(Taxonomy::new("Passeres"))
			)
			.add_child(Taxonomy::new("Amphibia")
				.add_child(Taxonomy::new("Reptiles"))
				.add_child(Taxonomy::new("Serpentes"))
				.add_child(Taxonomy::new("Nantes"))
			)
			.add_child(Taxonomy::new("Pisces"))
			.add_child(Taxonomy::new("Insecta"))
			.add_child(Taxonomy::new("Vermes")
				.add_child(Taxonomy::new("Intestina"))
				.add_child(Taxonomy::new("Mollusca"))
				.add_child(Taxonomy::new("Testacea"))
				.add_child(Taxonomy::new("Lithophyta"))
				.add_child(Taxonomy::new("Zoophyta"))
			)
		)
		.add_child(Taxonomy::new("Vegetalia")
			.add_child(Taxonomy::new("Monandria"))
			.add_child(Taxonomy::new("Diandria"))
			.add_child(Taxonomy::new("Triandria"))
			.add_child(Taxonomy::new("Tetrandria"))
			.add_child(Taxonomy::new("Pentandria"))
			.add_child(Taxonomy::new("Hexandria"))
			.add_child(Taxonomy::new("Heptandria"))
		)
		.add_child(Taxonomy::new("Mineralia")
			.add_child(Taxonomy::new("Petræ"))
			.add_child(Taxonomy::new("Mineræ"))
			.add_child(Taxonomy::new("Fossilia"))
			.add_child(Taxonomy::new("Vitamentra"))
		);

	// start the application
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(taxonomy)
        .expect("launch failed");
}

fn ui_builder() -> impl Widget<Taxonomy> {
	Scroll::new(Tree::new())
}
