import foo

class MyApp:
    def __init__(self):
        self.count = 42

    def run(self):
        print("Call into Python")
        return foo.button("hello from Python")

my_app = MyApp()

foo.ui(my_app.run)
