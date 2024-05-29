import foo

class MyApp:
    def __init__(self):
        self.count = 0

    def run(self, data):
        return foo.column(
            foo.button(f'count: {self.count}', self.handle_click),
            foo.button("reset", self.reset)
        )

    def handle_click(self, data):
        self.count += 1

    def reset(self, data):
        self.count = 0

my_app = MyApp()

foo.ui('this data is threaded down', my_app.run)
